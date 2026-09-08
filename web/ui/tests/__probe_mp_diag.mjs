import fs from "node:fs";
import {
  activateLocalButton,
  chromium,
  closePeerServer,
  freePort,
  sleep,
  startFullUiPeerMatch,
  startHarnessServer,
  startPeerServer,
  waitForLocalButton,
} from "./peerjs-resync-harness.js";

const OUT = process.env.OUT;
const peerPort = await freePort();
const peerServer = await startPeerServer(peerPort);
const { vite, baseUrl } = await startHarnessServer(peerPort);
const browser = await chromium.launch();
const hostContext = await browser.newContext({ viewport: { width: 1600, height: 1000 } });
const guestContext = await browser.newContext({ viewport: { width: 1600, height: 1000 } });
const dump = (page) => page.evaluate(() => {
  const s = window.__ironsmithDiagnostics.snapshot();
  return {
    traces: s.traces.slice(0, 3).map((t) => ({ label: t.label, mode: t.mode, total: t.totalMs && Math.round(t.totalMs), outcome: t.outcome, stages: t.stages.map((x) => `${x.name}+${Math.round(x.sincePreviousMs)}`) })),
    peers: s.peers.map((p) => ({ id: p.peerId.slice(0, 8), state: p.state, rtt: p.rttMs, avg: p.rttAvgMs && Math.round(p.rttAvgMs), inMs: p.sinceReceivedMs && Math.round(p.sinceReceivedMs), recv: p.received, sent: p.sent, kb: Math.round((p.bytesIn + p.bytesOut) / 1024) })),
    events: s.events.slice(0, 14).map((e) => `${e.kind} ${e.meta ? JSON.stringify(e.meta).slice(0, 140) : ""}`),
    mainThread: { lag: Math.round(s.mainThread.lagMs), worst: Math.round(s.mainThread.worstStallMs) },
    engine: s.engine && { queue: s.engine.queueWaitMs, wasm: s.engine.wasmCallMs },
  };
});
try {
  const { hostPage, guestPage } = await startFullUiPeerMatch({ baseUrl, hostContext, guestContext, securityMode: process.env.SECURITY_MODE || "" });
  const errors = [];
  for (const [label, page] of [["host", hostPage], ["guest", guestPage]]) page.on("pageerror", (e) => errors.push(`${label}: ${String(e).slice(0, 200)}`));
  await waitForLocalButton(hostPage, /Mulligan/i, "host mulligan local", 120000);
  await activateLocalButton(hostPage, "host-mulligan", /Mulligan/i);
  await sleep(6000);
  const report = { host: await dump(hostPage), guest: await dump(guestPage) };
  // Open the sheet on the host and screenshot it.
  const trigger = hostPage.locator("button", { hasText: /^Diagnostics$/i }).first();
  await trigger.click().catch(() => {});
  await sleep(1200);
  await hostPage.screenshot({ path: `${OUT}/diagnostics-mp.png` });
  report.sheet = (await hostPage.locator(".diagnostics-body").innerText().catch(() => "no sheet")).slice(0, 1600).replace(/\n+/g, " | ");
  report.errors = errors;
  fs.writeFileSync(`${OUT}/mp-diag.json`, JSON.stringify(report, null, 1));
} catch (err) {
  fs.writeFileSync(`${OUT}/mp-diag.json`, JSON.stringify({ failed: String(err?.stack || err) }, null, 1));
} finally {
  await browser.close().catch(() => {});
  await vite.close().catch(() => {});
  await closePeerServer(peerServer).catch(() => {});
  process.exit(0);
}
