// Real catalog opening controls. Midgame mechanic coverage is tracked separately.
import { build } from 'vite';
import { publicationIdentity, attributePublication } from './catalog-publication-timing.mjs';
import { runVerifiedNinjutsu, runVerifiedSneak } from './verified-ninjutsu-policy.mjs';
import { runVerifiedBroodscale } from './verified-broodscale-policy.mjs';
import http from 'node:http';
import path from 'node:path';
import { createHash } from 'node:crypto';
import { gzipSync } from 'node:zlib';
import { readFileSync, readdirSync, writeFileSync } from 'node:fs';
import {
  assert, chromium, freePort, startPeerServer, closePeerServer, UI_ROOT,
  startFullUiPeerMatch, waitForFullUiPair as waitForFullUiPairWithPolling, assertNoPageErrors,
  assertNoFullUiSyncFailuresWithDebug, test,
} from './peerjs-resync-harness.js';

// Functional tests use 250ms polling. Performance measurements need a finer
// observer interval; retain the exact readiness predicate and the full caller
// clock, including observer overhead, alongside both peers' publication times.
const waitForFullUiPair = (host, guest, predicate, label, timeout = 60000) =>
  waitForFullUiPairWithPolling(host, guest, predicate, label, timeout, 16);

const pkg = process.env.IRONSMITH_TEST_WASM_PKG;
const ninjutsuSeat = process.env.STRESS_NINJUTSU_SEAT === undefined ? null : Number(process.env.STRESS_NINJUTSU_SEAT);
assert.ok(ninjutsuSeat === null || ninjutsuSeat === 0 || ninjutsuSeat === 1);
const sneakSeat = process.env.STRESS_SNEAK_SEAT === undefined ? null : Number(process.env.STRESS_SNEAK_SEAT);
assert.ok(sneakSeat === null || sneakSeat === 0 || sneakSeat === 1);
assert.ok(ninjutsuSeat === null || sneakSeat === null, 'select one return-cost mechanic per run');
const returnCostSeat = ninjutsuSeat ?? sneakSeat;
const broodSeat = process.env.STRESS_BROOD_SEAT === undefined ? null : Number(process.env.STRESS_BROOD_SEAT);
assert.ok(broodSeat === null || broodSeat === 0 || broodSeat === 1);
assert.ok(broodSeat === null || returnCostSeat === null, 'Select one mechanic per run');
const mechanicMaxSearchTurn = Number(process.env.STRESS_MECHANIC_MAX_TURN || (broodSeat === null ? 40 : 100));
assert.ok(Number.isSafeInteger(mechanicMaxSearchTurn) && mechanicMaxSearchTurn >= 1 && mechanicMaxSearchTurn <= 100,
  'STRESS_MECHANIC_MAX_TURN must be an integer from 1 through 100');
// Natural draws, cleanup and phase transitions grow the authenticated history.
// This is a history-growth control; mechanic-heavy boards have separate fixtures.
const minTurn = Number(process.env.STRESS_MIN_TURN || 0);
assert.ok(Number.isSafeInteger(minTurn) && minTurn >= 0 && minTurn <= 40,
  'STRESS_MIN_TURN must be an integer from 0 through 40');
const minLandPlays = Number(process.env.STRESS_MIN_LAND_PLAYS || 1);
const activateMana = process.env.STRESS_ACTIVATE_MANA === '1';
assert.ok(Number.isSafeInteger(minLandPlays) && minLandPlays >= 1 && minLandPlays <= 8,
  'STRESS_MIN_LAND_PLAYS must be an integer from 1 through 8');
const battlefieldCount = player => player.battlefield.reduce((n, card) => n + (card.count || 1), 0);
const manaTotal = player => Object.values(player.mana_pool).reduce((sum, amount) => sum + amount, 0);
const tappedCount = player => player.battlefield.reduce((n, card) => n + (card.tapped ? card.count || 1 : 0), 0);
// Transcript acceptance and React publication are separate asynchronous steps.
const samePublishedStep = (a, b) => a.state.turn_number === b.state.turn_number
  && a.state.phase === b.state.phase && a.state.step === b.state.step
  && a.state.decision?.kind === b.state.decision?.kind
  && a.state.decision?.player === b.state.decision?.player;
const inventory = JSON.parse(readFileSync(new URL('../../../reports/performance/stress-suite/catalog-inventory.json', import.meta.url)));
const pairs = [
  ['standard', sneakSeat === null ? 'mtgtop8-91184-892571' : 'mtgtop8-90993-891010', 'mtgtop8-90920-890509'],
  ['pioneer', 'mtgtop8-90797-889463', 'mtgtop8-90797-889462'],
  ['modern', 'mtgtop8-91199-892709', 'mtgtop8-91199-892711'],
].filter(([format]) => !process.env.STRESS_FORMAT || process.env.STRESS_FORMAT === format);
assert.ok(returnCostSeat === null || pairs.every(([format]) => format === 'standard'));
assert.ok(broodSeat === null || pairs.every(([format]) => format === 'modern'));
assert.ok((minLandPlays === 1 && !activateMana) || pairs.every(([format]) => format === 'standard'),
  'the growing-land control currently supports the Standard pair; sacrifice/bounce lands need separate outcome assertions');

function commandTimingPlugin() {
  return { name: 'catalog-command-timing', enforce: 'pre', transform(source, id) {
    if (id.split('?')[0].endsWith('/context/GameContext.jsx')) {
      const anchor = 'priorityAnalysis: () => game?.latestPriorityAnalysis?.() || null,';
      assert.ok(source.includes(anchor), 'priority publication state capture anchor');
      const publishAnchor = 'window.__ironsmithE2E = e2eApi;';
      assert.ok(source.includes(publishAnchor), 'render publication timing anchor');
      const measured = source.replace(publishAnchor, `${publishAnchor}
      const publicationStarted = performance.now();
      const publicationRow = {
        schemaVersion: 2,
        at: performance.timeOrigin + publicationStarted,
        sequence: multiplayer.lastAppliedSequence,
        pendingVerification: Boolean(multiplayer.pendingVerification),
        identity: (${publicationIdentity.toString()})(state),
      };
      const publications = window.__catalogRenderedPublications ||= [];
      const previousPublication = publications.at(-1);
      if (!previousPublication || previousPublication.sequence !== publicationRow.sequence
          || previousPublication.pendingVerification !== publicationRow.pendingVerification
          || previousPublication.identity !== publicationRow.identity) {
        publicationRow.captureMs = performance.now() - publicationStarted;
        publications.push(publicationRow);
      }
      `);
      return { code: measured.replace(anchor, `${anchor}
      priorityPublicationState: () => {
        const describe = snapshot => ({ revision: snapshot?.__priority_revision,
          sequence: snapshot?.__priority_analysis_sequence,
          player: snapshot?.decision?.player, complete: snapshot?.decision?.analysis_complete,
          actions: snapshot?.decision?.actions?.length });
        return { rendered: describe(state), authoritative: describe(stateRef.current) };
      },`), map: null };
    }
    if (id.split('?')[0].endsWith('/workers/wasmGameWorker.js')) {
      const anchor = 'const raw = game.registerExternalCardSourcesJson(JSON.stringify(sources));';
      assert.ok(source.includes(anchor), 'batched registration timing anchor');
      const fetchAnchor = 'async function fetchCardSourceUncached(name) {';
      assert.ok(source.includes(fetchAnchor), 'worker card source timing anchor');
      const measuredSource = source.replace(fetchAnchor, `async function fetchCardSourceUncached(name) {
        const started = performance.now();
        try { return await measuredFetchCardSourceUncached(name); }
        finally { self.postMessage({ type: 'catalogCardSourceTiming', timing: {
          route: cardRouteKey(name), startedAt: performance.timeOrigin + started,
          durationMs: performance.now() - started } }); }
      }
      async function measuredFetchCardSourceUncached(name) {`);
      return { code: measuredSource.replace(anchor, `workerTasks.phaseActive('card_registration_encode');
      const encodedSources = JSON.stringify(sources);
      workerTasks.phaseActive('card_registration_engine');
      const raw = game.registerExternalCardSourcesJson(encodedSources);
      workerTasks.phaseActive('card_registration_decode');`), map: null };
    }
    if (id.split('?')[0].endsWith('/workers/paymentOptionsWorker.js')) {
      const replacements = [
        ['  try {', `  const measuredCalls = [];
  let previous = performance.now();
  const mark = method => {
    const now = performance.now();
    measuredCalls.push({stage: 'done', worker: 'paymentOptions', token: data.token,
      method, durationMs: now - previous, at: performance.timeOrigin + now});
    previous = now;
  };
  try {`],
        ['    const game = await replica.hydrate', "    mark('initializePaymentRuntime');\n    const game = await replica.hydrate"],
        ['    const replayMs =', "    mark('replayPaymentRuntime');\n    const replayMs ="],
        ['    const result = game.getPaymentActivationOptions(data.request);',
          "    mark('paymentReplayComplete');\n    const result = game.getPaymentActivationOptions(data.request);\n    mark('getPaymentActivationOptions');"],
        ['self.postMessage({ token: data.token, result:',
          'self.postMessage({ token: data.token, timings: measuredCalls, result:'],
      ];
      let code = source;
      for (const [before, after] of replacements) {
        assert.ok(code.includes(before), `payment timing anchor: ${before}`);
        code = code.replace(before, after);
      }
      return { code, map: null };
    }
    if (id.split('?')[0].endsWith('/lib/payment-options-analysis.js')) {
      const anchor = 'worker.onmessage = ({ data }) => {';
      assert.ok(source.includes(anchor), 'payment timing relay anchor');
      return { code: source.replace(anchor, `${anchor}
        for (const timing of data.timings || []) {
          self.postMessage({type: 'priorityAnalysisTiming', timing});
        }`), map: null };
    }
    if (id.split('?')[0].endsWith('/workers/priorityAnalysisWorker.js')) {
      const anchor = "const phase = value => self.postMessage({ type: 'phase', token, phase: value });";
      assert.ok(source.includes(anchor), 'isolated analysis timing anchor');
      let code = source.replace(anchor, `${anchor}
        const measuredAnalysisCall = (method, ...args) => {
          const started = performance.now();
          self.postMessage({type: 'analysisTiming', stage: 'start', token, method,
            at: performance.timeOrigin + started});
          let result;
          try { result = game[method](...args); return result; }
          finally { self.postMessage({type: 'analysisTiming', stage: 'done', token, method,
            accepted: method === 'beginPriorityAnalysis' ? result : undefined,
            complete: result?.analysis_complete, actions: result?.actions?.length,
            durationMs: performance.now() - started, at: performance.timeOrigin + performance.now()}); }
        };`);
      for (const [before, after] of [
        ['game.beginPriorityAnalysis(String(token))', "measuredAnalysisCall('beginPriorityAnalysis', String(token))"],
        ['game.stepPriorityAnalysis(String(token), 8)', "measuredAnalysisCall('stepPriorityAnalysis', String(token), 8)"],
        ['game.beginInspectorAnalysis(searchToken, ...request.args)', "measuredAnalysisCall('beginInspectorAnalysis', searchToken, ...request.args)"],
        ['game.stepInspectorAnalysis(searchToken, 8)', "measuredAnalysisCall('stepInspectorAnalysis', searchToken, 8)"],
      ]) {
        assert.ok(code.includes(before), `analysis call anchor: ${before}`);
        code = code.replace(before, after);
      }
      return { code, map: null };
    }
    if (id.split('?')[0].endsWith('/lib/isolated-priority-analysis.js')) {
      const anchor = 'worker.onmessage = ({ data }) => {';
      assert.ok(source.includes(anchor), 'analysis timing relay anchor');
      return { code: source.replace(anchor, `${anchor}
        if (data.type === 'analysisTiming') {
          self.postMessage({type: 'priorityAnalysisTiming', timing: data}); return;
        }`), map: null };
    }
    if (id.split('?')[0].endsWith('/hooks/peer-lobby/shared.js')) {
      const anchor = 'window.__ironsmithPerfEvents = shared.slice(-200);';
      assert.ok(source.includes(anchor), 'peer phase collection source anchor');
      return { code: source.replace(anchor, `${anchor}
        (window.__catalogPeerEvents ||= []).push(event);`), map: null };
    }
    if (!id.split('?')[0].endsWith('/hooks/useWasmGame.js')) return null;
    const anchor = 'const journalEntry = beginJournalEntry(method, args, { runtimeBranch });';
    const result = 'resolve: (value) => { completeJournalEntry(journalEntry, value); resolve(value); },';
    const failure = 'reject: (error) => { failJournalEntry(journalEntry, error); reject(error); },';
    const messageAnchor = 'const onMessage = (event) => {\n      if (disposed) return;\n      const msg = event.data || {};';
    assert.ok([anchor, result, failure].every(text => source.includes(text)), 'command timing source anchors');
    assert.ok(source.includes(messageAnchor), 'priority publication observation anchor');
    const cryptoCallAnchor = 'return callZiffleWorker(method, args);';
    assert.ok(source.includes(cryptoCallAnchor), 'crypto caller timing anchor');
    const withCryptoTiming = source.replace(cryptoCallAnchor, `
      const started = performance.now();
      const record = (error) => (window.__catalogCommandTimings ||= []).push({
        method, workerKind: 'ziffle', durationMs: performance.now() - started,
        cardPositionCount: Array.isArray(args[0]?.cardPositions) ? args[0].cardPositions.length
          : args[0]?.cardPosition == null ? null : 1,
        ...(error ? {error: String(error)} : {}),
        at: performance.timeOrigin + performance.now(),
      });
      return callZiffleWorker(method, args).then(value => { record(); return value; },
        error => { record(error); throw error; });`);
    return { code: withCryptoTiming.replace(messageAnchor, `${messageAnchor}
      if (msg.type === 'catalogCardSourceTiming') {
        (window.__catalogCardSourceTimings ||= []).push(msg.timing); return;
      }
      if (msg.type === 'workerDiagnostics' && msg.completed?.elapsedMs >= 100) {
        (window.__catalogSlowWorkerTasks ||= []).push(msg.completed);
      }
      if (msg.type === 'priorityAnalysis' || msg.type === 'priorityAnalysisError') {
        (window.__catalogPriorityEvents ||= []).push({type: msg.type, revision: msg.revision,
          sequence: msg.sequence, player: msg.decision?.player, complete: msg.decision?.analysis_complete,
          actions: msg.decision?.actions?.length, error: msg.error,
          at: performance.timeOrigin + performance.now()});
      }
      if (msg.type === 'priorityAnalysisTiming') {
        (window.__catalogAnalysisCalls ||= []).push(msg.timing);
      }`).replace(anchor, `${anchor}
      const __started = performance.now();`)
      .replace(result, `resolve: (value) => {
        (window.__catalogCommandTimings ||= []).push({method, runtimeBranch,
          durationMs: performance.now() - __started,
          worker: value?.__perf || null,
          at: performance.timeOrigin + performance.now()});
        completeJournalEntry(journalEntry, value); resolve(value);
      },`).replace(failure, `reject: (error) => {
        (window.__catalogCommandTimings ||= []).push({method, runtimeBranch,
          durationMs: performance.now() - __started, error: String(error),
          at: performance.timeOrigin + performance.now()});
        failJournalEntry(journalEntry, error); reject(error);
      },`), map: null };
  }};
}

async function productionServer(peerPort, outDir) {
  const immutablePackage = () => ({ name: 'catalog-immutable-wasm', enforce: 'pre',
    resolveId(source) {
      const match = source.match(/(?:^|\/)wasm_demo\/pkg\/(.+)$/);
      return match ? path.join(pkg, match[1]) : null;
    } });
  const env = { VITE_E2E_TEST: 'true', VITE_PEER_HOST: '127.0.0.1',
    VITE_PEER_PORT: String(peerPort), VITE_PEER_PATH: '/peerjs', VITE_PEER_KEY: 'peerjs',
    VITE_PEER_SECURE: 'false', VITE_PEER_HEARTBEAT_INTERVAL_MS: '500',
    VITE_PEER_HEARTBEAT_TIMEOUT_MS: '2000' };
  const prior = Object.fromEntries(Object.keys(env).map(key => [key, process.env[key]]));
  Object.assign(process.env, env);
  try {
    await build({ root: UI_ROOT, configFile: path.join(UI_ROOT, 'vite.config.js'), logLevel: 'warn',
      plugins: [commandTimingPlugin(), immutablePackage()],
      worker: { plugins: () => [commandTimingPlugin(), immutablePackage()] },
      // Serve catalog bytes from their canonical public directory. Copying the
      // entire catalog into each diagnostic build costs nearly a gigabyte;
      // observed response hashes below still record the exact input bytes.
      build: { outDir, emptyOutDir: true, assetsInlineLimit: 0, copyPublicDir: false },
    });
  } finally {
    for (const [key, value] of Object.entries(prior)) {
      if (value === undefined) delete process.env[key]; else process.env[key] = value;
    }
  }
  const digest = file => createHash('sha256').update(readFileSync(file)).digest('hex');
  const javascriptAssets = readdirSync(path.join(outDir, 'assets')).filter(file => file.endsWith('.js'))
    .map(file => ({ file, sha256: digest(path.join(outDir, 'assets', file)) }));
  const wasmAssets = readdirSync(path.join(outDir, 'assets')).filter(file => file.endsWith('.wasm'))
    .map(file => ({ file, sha256: digest(path.join(outDir, 'assets', file)) }));
  for (const file of ['engine_bg.wasm', 'compiler_bg.wasm', 'verifier_bg.wasm']) {
    assert.ok(wasmAssets.some(asset => asset.sha256 === digest(path.join(pkg, file))),
      `production build must contain the exact immutable ${file}`);
  }
  const types = { '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css',
    '.wasm': 'application/wasm', '.json': 'application/json', '.svg': 'image/svg+xml' };
  const publicDir = path.join(UI_ROOT, 'public');
  const server = http.createServer((req, res) => {
    const uri = decodeURIComponent(new URL(req.url, 'http://local').pathname);
    const file = path.resolve(outDir, '.' + (uri === '/' ? '/index.html' : uri));
    if (!file.startsWith(outDir + path.sep)) { res.writeHead(403).end(); return; }
    try {
      let bytes;
      try { bytes = readFileSync(file); }
      catch (error) {
        if (error.code !== 'ENOENT') throw error;
        const publicFile = path.resolve(publicDir, '.' + uri);
        if (!publicFile.startsWith(publicDir + path.sep)) throw error;
        bytes = readFileSync(publicFile);
      }
      res.writeHead(200, { 'Content-Type': types[path.extname(file)] || 'application/octet-stream' });
      res.end(bytes);
    }
    catch { res.writeHead(404).end(); }
  });
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  return { baseUrl: `http://127.0.0.1:${server.address().port}`, wasmAssets, javascriptAssets,
    close: () => new Promise(resolve => server.close(resolve)) };
}

test('production Verified catalog opening controls, both seats and matching audits',
  { skip: !pkg, timeout: Math.max(900000, mechanicMaxSearchTurn * 40000) }, async t => {
  const peerPort = await freePort();
  const peerServer = await startPeerServer(peerPort); t.after(() => closePeerServer(peerServer));
  const outDir = path.resolve(process.env.STRESS_BUILD_DIR || '/tmp/ironsmith-verified-catalog-production');
  const server = await productionServer(peerPort, outDir); t.after(() => server.close());
  const browser = await chromium.launch(); t.after(() => browser.close());
  const report = { mode: 'production UI + real two-client Verified', browser: browser.version(),
    observationPollIntervalMs: 16, cryptoWorkerCallerTiming: true,
    pkg, wasmAssets: server.wasmAssets, javascriptAssets: server.javascriptAssets,
    engineSha256: createHash('sha256').update(readFileSync(path.join(pkg, 'engine_bg.wasm'))).digest('hex'), cases: [] };
  t.after(() => {
    const output = process.env.STRESS_OUTPUT || '/tmp/verified-catalog-performance.json';
    const bytes = JSON.stringify(report, null, 2);
    writeFileSync(output, output.endsWith('.gz') ? gzipSync(bytes) : bytes);
  });
  for (const [format, ...ids] of pairs) {
    if (returnCostSeat === 1) ids.reverse();
    if (broodSeat === 0) ids.reverse();
    const decks = ids.map(id => inventory.formats[format].decks.find(deck => deck.id === id));
    assert.ok(decks.every(Boolean));
    const contexts = await Promise.all([0, 1].map(() => browser.newContext({ viewport: { width: 1600, height: 1000 } })));
    const row = { format, minTurn, minLandPlays, activateMana, ninjutsuSeat, sneakSeat, broodSeat, deckIds: ids, deckHashes: decks.map(d => d.sha256), interactions: [], peerEvents: [] };
    const cardResponseCaptures = new Set();
    row.observedCardAssetResponses = [];
    for (const [seat, context] of contexts.entries()) {
      await context.addInitScript(() => {
        const supported = PerformanceObserver.supportedEntryTypes.includes('longtask');
        const entries = [];
        const append = records => entries.push(...records.map(entry => ({
          ...entry.toJSON(), at: performance.timeOrigin + entry.startTime,
        })));
        const observer = supported ? new PerformanceObserver(list => append(list.getEntries())) : null;
        observer?.observe({ type: 'longtask', buffered: true });
        window.__catalogReadLongTasks = () => {
          if (observer) append(observer.takeRecords());
          return { supported, timeOrigin: performance.timeOrigin, entries };
        };
      });
      context.on('response', response => {
        const url = new URL(response.url());
        if (url.origin !== new URL(server.baseUrl).origin
          || !/^\/cards\/.*\.json$/.test(url.pathname)) return;
        const entry = { seat, path: url.pathname + url.search, status: response.status() };
        row.observedCardAssetResponses.push(entry);
        const capture = response.body().then(bytes => {
          entry.bytes = bytes.length;
          entry.sha256 = createHash('sha256').update(bytes).digest('hex');
          // Browser resource timing separates network service/body transfer
          // from the worker's subsequent parsing and registration phases.
          // Concurrent request intervals overlap; never sum their durations.
          entry.resourceTiming = response.request().timing();
          entry.fromServiceWorker = response.fromServiceWorker();
        }).catch(error => { entry.error = String(error); });
        cardResponseCaptures.add(capture);
        void capture.finally(() => cardResponseCaptures.delete(capture));
      });
    }
    report.cases.push(row);
    let pages = [];
    const cpuSessions = [];
    try {
      console.log(`[catalog ${format}] creating Verified match: ${ids.join(' vs ')}`);
      for (const context of contexts) {
        await context.route('**/lobbies', route => route.fulfill({ contentType: 'application/json', body: '{"lobbies":[]}', headers: { 'Access-Control-Allow-Origin': '*' } }));
      }
      const { hostPage: host, guestPage: guest } = await startFullUiPeerMatch({ baseUrl: server.baseUrl,
        hostContext: contexts[0], guestContext: contexts[1], securityMode: 'verified',
        hostDeckText: decks[0].mainboard.map(c => `${c.count} ${c.name}`).join('\n'),
        guestDeckText: decks[1].mainboard.map(c => `${c.count} ${c.name}`).join('\n') });
      pages = [host, guest];
      const initialAudits = await Promise.all(pages.map(page => page.evaluate(() => window.__ironsmithE2E.auditTranscript())));
      for (const audit of initialAudits) {
        for (const [seat, deck] of decks.entries()) {
          const expected = deck.mainboard.flatMap(card => Array(card.count).fill(card.name)).sort();
          assert.deepEqual([...audit.match.players[seat].deck].sort(), expected,
            `Verified seat ${seat} must retain the exact catalog deck without substitutions`);
        }
      }
      row.initialCatalogDecksVerified = true;
      if (activateMana || returnCostSeat !== null || broodSeat !== null) {
        // Keep the mana visible until its public outcome is checked, rather
        // than allowing automatic passes to empty the pool at a phase boundary.
        for (const page of pages) {
          const hold = page.getByRole('button', { name: /^(Hold|Holding) priority$/ });
          if (await hold.getAttribute('aria-pressed') !== 'true') await hold.click();
          assert.equal(await hold.getAttribute('aria-pressed'), 'true');
        }
      }
      row.consoleErrors = [];
      pages.forEach((page, seat) => page.on('console', message => {
        if (['error', 'warning'].includes(message.type())) {
          row.consoleErrors.push({ seat, type: message.type(), text: message.text(), at: Date.now() });
        }
      }));
      row.timeOrigins = await Promise.all(pages.map(page => page.evaluate(() => performance.timeOrigin)));
      // Capture both authenticated starting perspectives for reproducibility of
      // cryptographic shuffles; these setup reads precede measured gameplay.
      row.initialCheckpoints = await Promise.all(pages.map(page => page.evaluate(() => window.__ironsmithE2E.publicCheckpoint())));
      // Retain setup/cold-start measurements before separating gameplay calls.
      // They must not disappear merely because the match became ready.
      row.setupTimings = await Promise.all(pages.map(page => page.evaluate(() => ({
        commands: window.__catalogCommandTimings || [],
        analysisCalls: window.__catalogAnalysisCalls || [],
        slowWorkerTasks: window.__catalogSlowWorkerTasks || [],
      }))));
      assert.ok(row.setupTimings.every(timing => timing.commands.some(call => call.workerKind === 'ziffle')),
        'both seats must record cold crypto-worker caller timings');
      if (process.env.STRESS_CPU_PROFILE_PREFIX) {
        row.cpuProfiling = { diagnosticOnly: true, samplingIntervalUs: 1000, seats: [] };
        for (const [seat, page] of pages.entries()) {
          const session = await page.context().newCDPSession(page);
          await session.send('Profiler.enable');
          await session.send('Profiler.setSamplingInterval', { interval: 1000 });
          const startedAtWall = await page.evaluate(() => performance.timeOrigin + performance.now());
          await session.send('Profiler.start');
          const entry = { seat, startedAtWall };
          row.cpuProfiling.seats.push(entry);
          cpuSessions.push({ session, entry });
        }
      }
      console.log(`[catalog ${format}] match ready; measuring gameplay`);
      await Promise.all(pages.map(page => page.evaluate(() => {
        window.__catalogCommandTimings = []; window.__catalogPeerEvents = []; window.__catalogPriorityEvents = []; window.__catalogAnalysisCalls = [];
        window.__catalogSlowWorkerTasks = [];
      })));
      if (broodSeat !== null) {
        await runVerifiedBroodscale({ pages, row, seat: broodSeat, waitForFullUiPair, samePublishedStep,
          maxSearchTurn: mechanicMaxSearchTurn });
      } else if (returnCostSeat !== null) {
        const runReturnCost = sneakSeat === null ? runVerifiedNinjutsu : runVerifiedSneak;
        await runReturnCost({ pages, row, seat: returnCostSeat, waitForFullUiPair, samePublishedStep,
          maxSearchTurn: mechanicMaxSearchTurn });
      } else {
      const played = new Set();
      const attemptedLand = new Set();
      const expectedBattlefieldCounts = [0, 0];
      const completedLandPlays = [0, 0];
      const manaActivated = [false, false];
      const pendingMana = [null, null];
      let reachedTurn = 0;
      for (let i = 0; i < Math.max(70, minTurn * 40, minLandPlays * 80)
        && (completedLandPlays.some(count => count < minLandPlays) || reachedTurn < minTurn
          || (activateMana && manaActivated.some(done => !done))); i++) {
        const readyWaitStarted = performance.now();
        const settled = await waitForFullUiPair(host, guest, (a, b) => {
          const current = a.state.decision?.player === 0 ? a : b;
          return a.multiplayer.lastAppliedSequence === b.multiplayer.lastAppliedSequence
            && !a.multiplayer.pendingVerification && !b.multiplayer.pendingVerification
            && samePublishedStep(a, b)
            && current.state.decision && (current.state.decision.kind !== 'priority'
              || current.state.priority_analysis_complete === true);
        }, 'catalog action menu available on acting seat', 30000);
        const actionMenuReadyObservationMs = performance.now() - readyWaitStarted;
        const actor = settled.host.state.decision.player;
        const current = actor === 0 ? settled.host : settled.guest;
        const state = current.state;
        assert.ok(Number.isSafeInteger(state.turn_number),
          'E2E snapshot must expose the engine turn number');
        const actions = state.decisionActions || [];
        const land = completedLandPlays[actor] < minLandPlays
          && actions.find(a => a.action_ref.kind === 'play_land');
        const mana = activateMana && !manaActivated[actor] && !pendingMana[actor]
          && completedLandPlays[actor] >= minLandPlays
          && actions.find(a => a.action_ref.kind === 'activate_mana_ability');
        const action = land || mana || actions.find(a => ['keep_opening_hand', 'continue_pregame', 'begin_game', 'pass_priority'].includes(a.action_ref.kind));
        const keepSurveil = state.decision.kind === 'select_objects' && state.decision.reason === 'Surveil';
        let command;
        if (state.decision.kind === 'attackers') command = { type: 'declare_attackers', declarations: [] };
        else if (state.decision.kind === 'blockers') command = { type: 'declare_blockers', declarations: [] };
        else if (state.decision.kind === 'select_objects' && state.decision.reason === 'Discard') {
          const count = Number(/^Discard (\d+) card/.exec(state.decision.description || '')?.[1]);
          assert.ok(Number.isSafeInteger(count) && count > 0, 'explicit cleanup discard count');
          const candidates = state.decisionCandidates.filter(candidate => candidate.legal !== false);
          assert.ok(candidates.length >= count);
          command = { type: 'select_objects', object_ids: candidates.slice(0, count).map(candidate => candidate.id) };
        }
        else if (keepSurveil) {
          assert.equal(state.decisionDetail.min, 0, 'surveil allows keeping all revealed cards');
          command = { type: 'select_objects', object_ids: [] };
        }
        else if (state.decision.kind === 'select_objects' && state.decisionDetail.min === 0) {
          // Opening controls may decline optional ETB selections, including an
          // empty legal candidate set. Never fabricate an object or skip a cost.
          command = { type: 'select_objects', object_ids: [] };
          (row.optionalChoicesDeclined ||= []).push({ source: state.decision.source_name,
            reason: state.decision.reason, turn: state.turn_number, actor });
        }
        else if (state.decision.kind === 'select_options') command = { type: 'select_options', option_indices: [state.decisionOptions.find(o => o.legal !== false).index] };
        else if (action) command = { type: 'priority_action', action_ref: action.action_ref };
        assert.ok(command, `Unhandled catalog control decision: ${JSON.stringify(state.decision)}`);
        const before = performance.now();
        const previousSequence = current.multiplayer.lastAppliedSequence;
        const submission = await pages[actor].evaluate(async command => {
          const start = performance.now();
          await window.__ironsmithE2E.submitMultiplayerCommand(command);
          return { submittedAt: performance.timeOrigin + start, submissionMs: performance.now() - start };
        }, command);
        const { submissionMs } = submission;
        const publicationStart = performance.now();
        if (land) {
          attemptedLand.add(actor);
          expectedBattlefieldCounts[actor] = battlefieldCount(state.players[actor]) + 1;
        }
        if (mana && !land) pendingMana[actor] = {
          total: manaTotal(state.players[actor]) + 1,
          tapped: tappedCount(state.players[actor]) + 1,
          source: mana.action_ref.source,
        };
        const result = await waitForFullUiPair(host, guest, (a, b) => a.multiplayer.lastAppliedSequence > previousSequence
          && a.multiplayer.lastAppliedSequence === b.multiplayer.lastAppliedSequence
          && !a.multiplayer.pendingVerification && !b.multiplayer.pendingVerification
          && samePublishedStep(a, b)
          && pendingMana.every((expected, seat) => !expected || a.state.decision?.kind !== 'priority'
            || [a, b].every(view => manaTotal(view.state.players[seat]) === expected.total
              && tappedCount(view.state.players[seat]) === expected.tapped))
          && [...attemptedLand].every(seat =>
            (a.state.decision?.kind !== 'priority' && b.state.decision?.kind !== 'priority')
            || (battlefieldCount(a.state.players[seat]) === expectedBattlefieldCounts[seat]
              && battlefieldCount(b.state.players[seat]) === expectedBattlefieldCounts[seat])),
        'both catalog peers verify action', 30000);
        row.interactions.push({ actor, seq: previousSequence + 1, command, ...submission,
          turn: state.turn_number,
          priorityRevision: state.priority_revision,
          actionMenuReadyObservationMs,
          observedSequence: result.host.multiplayer.lastAppliedSequence,
          publicationTargets: [result.host, result.guest].map(view => ({
            sequence: view.multiplayer.lastAppliedSequence,
            identity: publicationIdentity(view.state),
          })),
          publicationObservationMs: performance.now() - publicationStart,
          callerThroughVerificationMs: performance.now() - before });
        assert.equal(result.host.state.turn_number, result.guest.state.turn_number,
          'both peers publish the same turn after verification');
        if (keepSurveil) {
          for (const view of [result.host, result.guest]) {
            assert.equal(view.state.players[actor].library_size, state.players[actor].library_size,
              'keeping every surveilled card preserves library size');
            assert.equal(view.state.players[actor].graveyard_size, state.players[actor].graveyard_size,
              'keeping every surveilled card moves none to the graveyard');
          }
        }
        reachedTurn = result.host.state.turn_number;
        if (result.host.state.decision?.kind === 'priority') {
          for (const seat of [0, 1]) {
            if (!pendingMana[seat]) continue;
            assert.deepEqual(result.host.state.players[seat].mana_pool, result.guest.state.players[seat].mana_pool);
            manaActivated[seat] = true;
            pendingMana[seat] = null;
          }
        }
        console.log(`[catalog ${format}] seq ${previousSequence + 1} seat ${actor} ${command.action_ref?.kind || command.type}: submit ${submissionMs.toFixed(1)} ms`);
        for (const seat of attemptedLand) {
          // Some lands have an entry choice; assert once the action has completed.
          if (result.host.state.decision?.kind === 'priority') {
            assert.ok(result.host.state.players[seat].battlefield.length > 0);
            assert.ok(result.guest.state.players[seat].battlefield.length > 0);
            const publicCards = player => player.battlefield.map(card => [card.name, card.count || 1])
              .sort((a, b) => a[0].localeCompare(b[0]));
            assert.deepEqual(publicCards(result.host.state.players[seat]), publicCards(result.guest.state.players[seat]));
            completedLandPlays[seat] = expectedBattlefieldCounts[seat];
            played.add(seat);
          }
        }
      }
      assert.equal(played.size, 2, 'both seats play a land from their real catalog deck');
      assert.ok(completedLandPlays.every(count => count >= minLandPlays),
        `each seat must complete ${minLandPlays} natural land plays: ${completedLandPlays}`);
      row.completedLandPlays = completedLandPlays;
      row.manaActivated = manaActivated;
      if (activateMana) assert.ok(manaActivated.every(Boolean), 'both seats execute and publish a mana activation');
      assert.ok(reachedTurn >= minTurn, `natural progression reached turn ${reachedTurn}, expected ${minTurn}`);
      row.reachedTurn = reachedTurn;
      }
      const audits = await Promise.all(pages.map(page => page.evaluate(() => window.__ironsmithE2E.auditTranscript())));
      row.auditTranscripts = audits;
      assert.ok(audits[0].finalStateHash);
      assert.ok(audits[0].finalPublicCheckpointHash);
      assert.equal(audits[0].finalStateHash, audits[1].finalStateHash);
      assert.equal(audits[0].finalPublicCheckpointHash, audits[1].finalPublicCheckpointHash);
      row.auditHashes = audits.map(a => ({ state: a.finalStateHash, public: a.finalPublicCheckpointHash }));
      await Promise.all(cardResponseCaptures);
      const observedCardAssets = new Map();
      for (const asset of row.observedCardAssetResponses) {
        assert.ok(asset.status === 200 && !asset.error && /^[a-f0-9]{64}$/.test(asset.sha256 || ''),
          `complete card-source response required: ${asset.path}`);
        const resource = asset.path.split('?')[0];
        if (observedCardAssets.has(resource)) {
          assert.equal(asset.sha256, observedCardAssets.get(resource),
            `card source changed between reads or seats: ${resource}`);
        }
        observedCardAssets.set(resource, asset.sha256);
      }
      row.commands = await Promise.all(pages.map(page => page.evaluate(() => window.__catalogCommandTimings || [])));
      row.slowWorkerTasks = await Promise.all(pages.map(page => page.evaluate(() => window.__catalogSlowWorkerTasks || [])));
      assert.ok(row.commands.every(commands => commands.some(command => command.method === 'dispatch')),
        'both peers must execute measured gameplay commands');
      assert.ok(row.commands.every(commands => commands.some(call => call.workerKind === 'ziffle')),
        'both seats must record gameplay crypto-worker caller timings');
      assert.ok([...row.commands.flat(), ...row.setupTimings.flatMap(timing => timing.commands)]
        .every(call => Number.isFinite(call.durationMs) && call.durationMs >= 0),
        'every measured command must have a finite nonnegative caller duration');
      if (returnCostSeat !== null) {
        assert.ok(!row.commands[1 - returnCostSeat].some(call => call.method === 'getPaymentActivationOptions'),
          'observing peer must not enumerate uneditable payment alternatives');
      }
      row.peerEvents = await Promise.all(pages.map(page => page.evaluate(() => window.__catalogPeerEvents || [])));
      row.emptyPreviewReuseCounts = row.peerEvents.map(events => events.filter(event =>
        event.label === 'peer sync:submit_action:reuse_empty_preview').length);
      assert.ok(row.emptyPreviewReuseCounts.every(count => count > 0),
        'both seats reuse an empty crypto preview after introducing no material');
      row.worstCommandMs = row.commands.flat().reduce((max, call) => Math.max(max, call.durationMs || 0), 0);
      row.analysisCalls = await Promise.all(pages.map(page => page.evaluate(() => window.__catalogAnalysisCalls || [])));
      row.priorityEvents = await Promise.all(pages.map(page => page.evaluate(() => window.__catalogPriorityEvents || [])));
      assert.ok(row.priorityEvents.every(events => events.some(event => event.type === 'priorityAnalysis')),
        'both seats must observe priority analysis publications');
      assert.ok(row.analysisCalls.every(calls => calls.some(call => call.stage === 'done')),
        'both seats must report measured isolated analysis calls');
      row.worstAnalysisCallMs = row.analysisCalls.flat().reduce((max, call) => Math.max(max, call.durationMs || 0), row.worstAnalysisCallMs || 0);
      await assertNoFullUiSyncFailuresWithDebug('catalog controls preserve verification', host, guest);
      assertNoPageErrors(host, guest);
      assert.ok(row.worstCommandMs <= 800, `${format}: worst command ${row.worstCommandMs} ms exceeds 800 ms`);
      assert.ok(row.worstAnalysisCallMs <= 800, `${format}: isolated analysis call ${row.worstAnalysisCallMs} ms exceeds 800 ms`);
      const worstSetupMs = row.setupTimings.flatMap(timing => [
        ...(timing.commands || []), ...(timing.analysisCalls || []),
      ]).reduce((max, call) => Math.max(max, call.durationMs || 0), 0);
      assert.ok(worstSetupMs <= 800, `${format}: cold setup call ${worstSetupMs} ms exceeds 800 ms`);
      const worstInteractionMs = row.interactions.reduce((max, action) =>
        Math.max(max, action.callerThroughVerificationMs || 0), 0);
      assert.ok(worstInteractionMs <= 800,
        `${format}: complete interaction ${worstInteractionMs} ms exceeds 800 ms`);
      console.log(`[catalog ${format}] passed; worst worker call ${row.worstCommandMs.toFixed(1)} ms`);
    } catch (error) {
      row.error = String(error.stack || error);
      row.failureCheckpoints = await Promise.all(pages.map(page => page.evaluate(async () => {
        return Promise.race([
          window.__ironsmithE2E.publicCheckpoint(),
          new Promise(resolve => setTimeout(() => resolve({ error: 'checkpoint capture timed out' }), 5000)),
        ]);
      }).catch(error => ({ error: String(error) }))));
      throw error;
    }
    finally {
      for (const { session, entry } of cpuSessions) {
        try {
          const { profile } = await session.send('Profiler.stop');
          const file = `${process.env.STRESS_CPU_PROFILE_PREFIX}-${format}-seat${entry.seat}.cpuprofile`;
          const bytes = JSON.stringify(profile);
          writeFileSync(file, bytes);
          Object.assign(entry, { file, sha256: createHash('sha256').update(bytes).digest('hex'),
            stoppedAtWall: Date.now(), samples: profile.samples?.length ?? null });
        } catch (error) {
          entry.error = String(error);
        } finally {
          await session.detach().catch(() => {});
        }
      }
      if (pages.length) {
        row.mainThreadLongTasks = await Promise.all(pages.map((page, seat) => page.evaluate(() =>
          window.__catalogReadLongTasks?.() ?? null).catch(() => row.mainThreadLongTasks?.[seat] ?? null)));
        // Preserve the signed history as well as final hashes: checkpoints
        // alone cannot reconstruct a suspended effect or hidden-card replay.
        row.cardSourceTimings = await Promise.all(pages.map((page, seat) => page.evaluate(() => window.__catalogCardSourceTimings || []).catch(() => row.cardSourceTimings?.[seat] || [])));
        row.auditTranscripts ??= await Promise.all(pages.map(page => page.evaluate(() =>
          window.__ironsmithE2E.auditTranscript()).catch(error => ({ error: String(error) }))));
        row.analysisCalls = await Promise.all(pages.map((page, seat) => page.evaluate(() => window.__catalogAnalysisCalls || []).catch(() => row.analysisCalls?.[seat] || [])));
        row.worstAnalysisCallMs = row.analysisCalls.flat().reduce((max, call) => Math.max(max, call.durationMs || 0), row.worstAnalysisCallMs || 0);
        row.priorityEvents = await Promise.all(pages.map((page, seat) => page.evaluate(() => window.__catalogPriorityEvents || []).catch(() => row.priorityEvents?.[seat] || [])));
        row.finalPriorityAnalysis = await Promise.all(pages.map(page => page.evaluate(() => window.__ironsmithE2E.priorityAnalysis()).catch(() => null)));
        row.finalPriorityPublicationState = await Promise.all(pages.map(page => page.evaluate(() => window.__ironsmithE2E.priorityPublicationState()).catch(() => null)));
        row.finalSnapshots = await Promise.all(pages.map(page => page.evaluate(() => window.__ironsmithE2E.snapshot()).catch(() => null)));
        row.renderedPublications = await Promise.all(pages.map((page, seat) => page.evaluate(() => window.__catalogRenderedPublications || []).catch(() => row.renderedPublications?.[seat] || [])));
        for (const interaction of row.interactions) {
          interaction.renderPublication = attributePublication(interaction, row.renderedPublications);
        }
        // A test timeout can close a page during final diagnostics. Keep the
        // earlier successful capture rather than replacing evidence with [].
        row.commands = await Promise.all(pages.map((page, seat) => page.evaluate(() => window.__catalogCommandTimings || [])
          .catch(() => row.commands?.[seat] || [])));
        row.slowWorkerTasks = await Promise.all(pages.map((page, seat) => page.evaluate(() => window.__catalogSlowWorkerTasks || []).catch(() => row.slowWorkerTasks?.[seat] || [])));
        row.peerEvents = await Promise.all(pages.map((page, seat) => page.evaluate(() => window.__catalogPeerEvents || []).catch(() => row.peerEvents?.[seat] || [])));
        row.worstCommandMs = row.commands.flat().reduce((max, call) => Math.max(max, call.durationMs || 0), row.worstCommandMs || 0);
      }
      await Promise.all(contexts.map(context => context.close()));
      await Promise.all(cardResponseCaptures);
    }
  }
});
