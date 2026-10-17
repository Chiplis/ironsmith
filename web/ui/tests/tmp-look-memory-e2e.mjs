import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";
import { createServer } from "vite";
const out = process.argv[2];
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const scenarioApi = `const e2eApi = {
  scenarioState: () => state,
  setupLook: () => runWasmInteraction(async () => {
    setAutoPassEnabled(false);
    setHoldRule("always");
    await game.resetEmpty(["Alice", "Bob"], 20);
    for (let i = 0; i < 3; i++) await game.addCardToZone(0, "Island", "battlefield", true);
    for (const name of ["Peek", "Opt"]) await game.addCardToZone(0, name, "hand", true);
    for (const name of ["Lightning Bolt", "Counterspell", "Shock"]) await game.addCardToZone(1, name, "hand", true);
    for (const name of ["Plains", "Forest", "Mountain", "Swamp", "Plains", "Forest"]) await game.addCardToZone(0, name, "library", true);
    for (let i = 0; i < 6; i++) await game.addCardToZone(1, "Swamp", "library", true);
    await game.addCardToZone(1, "Grizzly Bears", "battlefield", true);
    await game.finishPuzzleSetup();
    let next = await game.uiState();
    for (let i = 0; i < 30; i++) {
      if (next.phase === "first main phase" && next.active_player === 0) break;
      const action = next.decision?.actions?.find(action =>
        ["keep_opening_hand", "begin_game", "continue_pregame", "pass_priority"].includes(action.kind));
      if (!action) throw new Error("Cannot advance scenario: " + JSON.stringify(next.decision));
      next = await game.dispatch({ type: "priority_action", action_ref: action.action_ref });
    }
    await finalizeState(game, next);
  }),`;
process.env.VITE_E2E_TEST = "true";
const server = await createServer({ root, logLevel: "error", server: { host: "127.0.0.1", port: 0, hmr: false },
  plugins: [{ name: "look-scenario", transform(code, id) {
    if (id.endsWith("/context/GameContext.jsx")) return code.replace("const e2eApi = {", scenarioApi);
  } }] });
await server.listen();
const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1600, height: 900 } });
page.on("pageerror", (e) => console.log("PAGEERROR", e.message));
const S = () => page.evaluate(() => window.__ironsmithE2E.scenarioState());
const D = (cmd) => page.evaluate(async (cmd) => {
  try { const r = await window.__ironsmithE2E.dispatch(cmd); return r === undefined ? "undef" : (typeof r === "object" ? Object.keys(r || {}).slice(0, 5).join(",") : String(r)); }
  catch (e) { return "ERR " + e.message; }
}, cmd).then((r) => { if (String(r).startsWith("ERR")) console.log("dispatch", cmd.type, r.slice(0, 300)); return r; });
async function settle() { await page.waitForTimeout(700); }
// Resolve whatever comes until priority with an empty stack.
async function drive(label) {
  for (let i = 0; i < 25; i++) {
    const s = await S();
    const d = s.decision;
    const stack = (s.stack_objects || s.stack_preview || []).length;
    if (d?.kind === "priority" && stack === 0 && i > 0) return;
    if (d?.kind === "mana_payment") await D({ type: "mana_payment", response: { action: "confirm", plan_id: String(s.mana_payment?.plan_id ?? d.plan_id), request_hash: String(s.mana_payment?.request_hash ?? d.request_hash) } });
    else if (d?.kind === "targets") await D({ type: "select_targets", targets: [{ kind: "player", player: 1 }] });
    else if (d?.kind === "select_options") await D({ type: "select_options", option_indices: [(d.options.find(o => /normal/i.test(o.description)) || d.options[0]).index] });
    else if (d?.kind === "priority") {
      const pass = d.actions.find(a => a.kind === "pass_priority");
      if (!pass) { console.log(label, "no pass", JSON.stringify(d.actions.map(a => a.kind))); return; }
      await D({ type: "priority_action", action_ref: pass.action_ref });
    } else { console.log(label, "stopped at", d?.kind, JSON.stringify(d).slice(0, 400)); return d?.kind; }
    await settle();
  }
}
async function cast(name) {
  for (let i = 0; i < 6; i++) {
    const cur = await S();
    if (cur.decision?.kind !== "priority" || cur.decision.player === 0) break;
    const pass = cur.decision.actions.find(a => a.kind === "pass_priority");
    await D({ type: "priority_action", action_ref: pass.action_ref });
    await settle();
  }
  const s = await S();
  const card = s.players[0].hand_cards.find(c => c.name === name);
  const action = s.decision?.actions?.find(a => a.kind === "cast_spell" && a.object_id === card?.id);
  if (!action) { console.log("cannot cast", name, s.phase, s.active_player, JSON.stringify(s.decision).slice(0, 600), JSON.stringify(s.players[0].hand_cards.map(c => [c.id, c.name]))); process.exit(1); }
  await D({ type: "priority_action", action_ref: action.action_ref });
  await page.waitForFunction(() => window.__ironsmithE2E.scenarioState()?.decision?.kind !== "priority"
    || (window.__ironsmithE2E.scenarioState()?.stack_objects || []).length > 0, null, { timeout: 15000 }).catch(() => console.log("cast did not leave priority"));
  await settle();
}
try {
  await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/`);
  await page.waitForFunction(() => window.__ironsmithE2E?.snapshot()?.loading === false, null, { timeout: 120000 });
  await page.waitForSelector(".game-card.hand-card", { timeout: 120000 });
  await page.waitForTimeout(3000);
  for (let attempt = 0; attempt < 3; attempt++) {
    await page.evaluate(() => window.__ironsmithE2E.setupLook());
    await page.waitForTimeout(2500);
    const hand = (await S()).players[0].hand_cards.map(c => c.name);
    console.log("seeded hand", hand.join(","));
    if (hand.includes("Peek")) break;
  }
  await page.keyboard.press("Escape");
  await settle();
  await cast("Peek");
  const stoppedPeek = await drive("peek");
  await page.screenshot({ path: `${out}/peek-during.png` });
  if (stoppedPeek) {
    // A look prompt: acknowledge it like the Done Looking button.
    const s = await S();
    console.log("peek view", JSON.stringify(s.viewed_cards?.cards?.map(c => [c.name, c.zone])));
  }
  let s = await S();
  console.log("after peek: decision", s.decision?.kind, "viewed", JSON.stringify(s.viewed_cards?.cards?.map(c => c.name)));
  console.log("bob looks", JSON.stringify(s.players[1].persistent_look_cards?.map(c => [c.name, c.zone, c.position])), "hand_size", s.players[1].hand_size);
  await page.mouse.move(800, 450);
  await page.waitForTimeout(800);
  await page.screenshot({ path: `${out}/after-peek.png` });
  await page.screenshot({ path: `${out}/after-peek-top.png`, clip: { x: 400, y: 0, width: 800, height: 140 } });
  const known = page.locator('.opponent-hand-card[data-known="true"]');
  console.log("known fan cards", await known.count());
  if (await known.count()) {
    const box = await known.nth(1).boundingBox();
    const top = Math.max(box.y, 0);
    console.log("known card box", JSON.stringify(box));
    const hx = box.x + box.width / 2, hy = top + (box.y + box.height - top) / 2;
    console.log("under pointer", await page.evaluate(({x, y}) => document.elementsFromPoint(x, y).slice(0, 4).map(e => e.className?.toString().slice(0, 60)).join(" | "), { x: hx, y: hy }));
    await page.mouse.move(hx, hy, { steps: 4 });
    await page.waitForTimeout(1200);
    const inspector = await page.evaluate(() => [...document.querySelectorAll('[data-card-hover-preview="true"][data-visible="true"], [data-card-inspector="true"]')].map(e => e.innerText.slice(0, 80)));
    console.log("inspector after hover", JSON.stringify(inspector), "previews", await page.evaluate(() => [...document.querySelectorAll('[data-card-hover-preview], [data-card-inspector]')].map(e => e.outerHTML.slice(0, 160)).join("\n")));
    await page.screenshot({ path: `${out}/hover-known.png` });
  }
  await page.mouse.move(800, 450);
  const done = page.getByRole("button", { name: /done looking/i }).first();
  if (await done.count()) { await done.click({ force: true }); await settle(); }
  s = await S();
  console.log("after done: viewed", JSON.stringify(s.viewed_cards?.cards?.map(c => c.name)), "bob looks", JSON.stringify(s.players[1].persistent_look_cards?.map(c => c.name)), "known fan", await known.count());
  await cast("Opt");
  const stoppedOpt = await drive("opt");
  s = await S();
  console.log("opt stop", stoppedOpt, "decision", s.decision?.kind, JSON.stringify(s.viewed_cards?.cards?.map(c => [c.name, c.zone, c.position])));
  await page.screenshot({ path: `${out}/opt-during.png` });
  console.log("alice looks", JSON.stringify(s.players[0].persistent_look_cards?.map(c => [c.name, c.zone, c.position])));
  console.log("eye chips", JSON.stringify(await page.locator('[data-zone-pile="look"]').evaluateAll(es => es.map(e => [e.dataset.lookZone, e.innerText]))));
  console.log("geometry", JSON.stringify(await page.evaluate(() => {
    const r = (e) => e && (({ top, bottom, left, right }) => ({ top: Math.round(top), bottom: Math.round(bottom), left: Math.round(left), right: Math.round(right) }))(e.getBoundingClientRect());
    const piles = document.querySelector('[data-local-zone-piles]');
    return { rows: [...document.querySelectorAll('[data-local-zone-piles] .zone-icon-row')].map(r), lift: piles?.style.getPropertyValue('--local-piles-lift'), pilesTop: piles?.style.getPropertyValue('--zone-piles-top'),
      docks: [...document.querySelectorAll('.battlefield-human-action-dock, .decision-dock, [data-decision-dock], .action-strip-decision-inline-summary')].map(e => [e.className.toString().slice(0, 50), r(e)]) };
  })));
  const eye = page.locator('[data-zone-pile="look"]').first();
  if (await eye.count()) {
    const eb = await eye.boundingBox();
    console.log("eye box", JSON.stringify(eb), "top element", await page.evaluate(({x, y}) => { const e = document.elementFromPoint(x, y); return e?.className?.toString().slice(0, 80); }, { x: eb.x + eb.width / 2, y: eb.y + eb.height / 2 }));
    await page.mouse.move(eb.x + eb.width / 2, eb.y + eb.height / 2);
    await page.waitForTimeout(900);
    console.log("eye strip", JSON.stringify(await page.locator('.zone-pile-menu--eye .zone-pile-card-row').evaluateAll(es => es.map(e => e.getAttribute("aria-label")))));
    await page.screenshot({ path: `${out}/eye-open.png` });
    await page.mouse.move(800, 450);
    const s2 = await S();
    const pass = (s2.decision?.actions || []).find(a => a.kind === "pass_priority");
    // Shuffle check: no shuffle effect handy here; covered by the native test.
  }
} finally {
  await browser.close();
  await server.close();
}
