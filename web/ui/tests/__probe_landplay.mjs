import fs from "node:fs";
import {
  buttonDebugText,
  chromium,
  clickLocalButton,
  clickLocalDecisionButton,
  closePeerServer,
  freePort,
  sleep,
  startFullUiPeerMatch,
  startHarnessServer,
  startPeerServer,
  visibleBodyText,
} from "./peerjs-resync-harness.js";

const OUT = process.env.OUT;
const peerPort = await freePort();
const peerServer = await startPeerServer(peerPort);
const { vite, baseUrl } = await startHarnessServer(peerPort);
const browser = await chromium.launch();
const hostContext = await browser.newContext({ viewport: { width: 1600, height: 1000 } });
const guestContext = await browser.newContext({ viewport: { width: 1600, height: 1000 } });
const log = [];
try {
  const { hostPage, guestPage } = await startFullUiPeerMatch({ baseUrl, hostContext, guestContext, hostDeckText: "60 Mountain", guestDeckText: "60 Mountain" });
  const errors = [];
  for (const [label, page] of [["host", hostPage], ["guest", guestPage]]) page.on("pageerror", (e) => errors.push(`${label}: ${String(e).slice(0, 200)}`));
  let hostPlayed = false;
  for (let step = 0; step < 40 && !hostPlayed; step += 1) {
    const play = await clickLocalButton(hostPage, "host-play", /PLAY MOUNTAIN/i);
    if (play) { hostPlayed = true; log.push({ step, side: "host", clicked: play.text, note: "PLAY MOUNTAIN" }); break; }
    const hostClicked = await clickLocalDecisionButton(hostPage, "host");
    if (hostClicked) { log.push({ step, side: "host", clicked: hostClicked.text }); await sleep(2200); continue; }
    const guestClicked = await clickLocalDecisionButton(guestPage, "guest");
    if (guestClicked) { log.push({ step, side: "guest", clicked: guestClicked.text }); await sleep(2200); continue; }
    log.push({ step, side: "none", hostLocal: (await buttonDebugText(hostPage)).filter((b) => b.localAction === "true").map((b) => b.text), hostBody: (await visibleBodyText(hostPage)).replace(/\s+/g, " ").slice(0, 300) });
    await sleep(1000);
  }
  const diag = await hostPage.evaluate(() => { const s = window.__ironsmithDiagnostics.snapshot(); return { traces: s.traces.slice(0, 6).map((t) => `${t.label} ${t.outcome} ${Math.round(t.totalMs || 0)}ms [${t.stages.map((x) => x.name).join(">")}]`), events: s.events.slice(0, 10).map((e) => e.kind + " " + JSON.stringify(e.meta || {}).slice(0, 120)) }; });
  fs.writeFileSync(`${OUT}/landplay.json`, JSON.stringify({ hostPlayed, log, diag, errors, hostBody: (await visibleBodyText(hostPage)).replace(/\s+/g, " ").slice(0, 600), guestBody: (await visibleBodyText(guestPage)).replace(/\s+/g, " ").slice(0, 600) }, null, 1));
} catch (err) {
  fs.writeFileSync(`${OUT}/landplay.json`, JSON.stringify({ failed: String(err?.stack || err), log }, null, 1));
} finally {
  await browser.close().catch(() => {});
  await vite.close().catch(() => {});
  await closePeerServer(peerServer).catch(() => {});
  process.exit(0);
}
