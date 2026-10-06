import test from "node:test";
import assert from "node:assert/strict";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";
import { createServer } from "vite";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const fixture = `import React from 'react';
import {createRoot} from 'react-dom/client';
import {GameProvider, useGame} from '/src/context/GameContext.jsx';
function Probe() {
  const context = useGame();
  window.context = context;
  return React.createElement('output', null, context.state?.snapshot_id);
}
createRoot(document.getElementById('root')).render(React.createElement(GameProvider, null, React.createElement(Probe)));`;
const peerMock = `import {useEffect} from 'react';
const multiplayer = {matchStarted:true, role:'client', submittingAction:false};
const submitMultiplayerCommand = (command) => new Promise((resolve,reject) => {
  window.submissions.push(command);
  window.pendingSubmission = {resolve,reject};
});
window.submissions = [];
export function usePeerLobby({setState}) {
  useEffect(() => {window.publish = setState;}, [setState]);
  return {multiplayer, submitMultiplayerCommand};
}`;
const wasmMock = `const game = {setAutoCleanupDiscard:async()=>{}};
export function useWasmGame() {return {game,loading:false};}`;

function priority(snapshot_id, overrides = {}) {
  return {
    snapshot_id, turn_number: 8, phase: "end phase", perspective: 0,
    active_player: 1, priority_player: 0, stack_size: 0,
    decision: { kind: "priority", player: 0, analysis_complete: true, actions: [
      { index: 0, kind: "pass_priority", label: "Pass priority", action_ref: { kind: "pass_priority" } },
    ] },
    ...overrides,
  };
}

test("GameProvider resumes auto-pass after automatic and manual submissions release their guard", { timeout: 60000 }, async () => {
  const server = await createServer({ root, logLevel: "error", server: { host: "127.0.0.1", port: 0, hmr: false }, plugins: [{
    name: "autopass-controlled-transport",
    transform(code, id) {
      if (id.endsWith("/hooks/usePeerLobby.js")) return peerMock;
      if (id.endsWith("/hooks/useWasmGame.js")) return wasmMock;
    },
    configureServer(server) {
      server.middlewares.use((request, response, next) => {
        if (request.url === "/autopass-fixture.html") {
          response.setHeader("Content-Type", "text/html");
          response.end('<div id="root"></div><script type="module" src="/autopass-fixture.jsx"></script>');
        } else next();
      });
    },
    resolveId(id) { if (id === "/autopass-fixture.jsx") return "\0autopass-fixture.jsx"; },
    load(id) { if (id === "\0autopass-fixture.jsx") return fixture; },
  }] });
  await server.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage();
    const errors = [];
    page.on("pageerror", error => errors.push(error.message));
    const load = async () => {
      await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/autopass-fixture.html`);
      await page.waitForFunction(() => window.publish && window.context);
    };
    const publish = async state => {
      await page.evaluate(state => window.publish(state), state);
      await page.waitForFunction(id => window.context.state?.snapshot_id === id, state.snapshot_id);
      // Drain effects without changing any GameProvider dependencies.
      await page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
    };
    const count = () => page.evaluate(() => window.submissions.length);
    const resolve = () => page.evaluate(() => window.pendingSubmission.resolve());
    await load();
    await publish(priority(1));
    assert.equal(await count(), 1);
    await publish(priority(2)); // Render while submittingAction is already false but the promise is still pending.
    assert.equal(await count(), 1);
    await resolve();
    await page.waitForFunction(() => window.submissions.length === 2, null, { timeout: 2000 });
    await resolve(); // Same snapshot must not cause an endless pass/retry loop.
    await publish(priority(2));
    assert.equal(await count(), 2);
    await publish(priority(3, { decision: { ...priority(3).decision, analysis_complete: false } }));
    assert.equal(await count(), 2);
    await publish(priority(3));
    assert.equal(await count(), 3);
    await publish(priority(4));
    await page.evaluate(() => window.context.setAutoPassEnabled(false));
    await resolve();
    await publish(priority(5));
    assert.equal(await count(), 3, "completion must respect the latest auto-pass setting");

    await load();
    await publish(priority(10, { active_player: 0 })); // Local empty-stack priority requires a manual pass.
    assert.equal(await count(), 0);
    await page.evaluate(() => { void window.context.dispatch({ type: "priority_action", action_ref: { kind: "pass_priority" } }); });
    await page.waitForFunction(() => window.submissions.length === 1);
    await publish(priority(11));
    assert.equal(await count(), 1);
    await resolve();
    await page.waitForFunction(() => window.submissions.length === 2, null, { timeout: 2000 });
    await publish(priority(12, { active_player: 0 }));
    await resolve();
    await publish(priority(12, { active_player: 0 }));
    assert.equal(await count(), 2, "completion must respect the latest hold reason");
    await publish(priority(13));
    assert.equal(await count(), 3);
    await publish(priority(14));
    await page.evaluate(() => window.pendingSubmission.reject(new Error("Controlled transport failure")));
    await page.waitForFunction(() => window.submissions.length === 4, null, { timeout: 2000 });
    await page.evaluate(() => window.pendingSubmission.reject(new Error("Controlled transport failure")));
    await publish(priority(14));
    assert.equal(await count(), 4, "a rejected attempt must release the guard without retrying the same snapshot forever");
    assert.deepEqual(errors, []);
  } finally {
    await browser.close();
    await server.close();
  }
});
