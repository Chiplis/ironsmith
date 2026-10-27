import test from "node:test";
import assert from "node:assert/strict";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";
import { createServer } from "vite";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const scenarioApi = `const e2eApi = {
  scenarioState: () => state,
  setupEquipment: () => runWasmInteraction(async () => {
    setAutoPassEnabled(false);
    setHoldRule("always");
    await game.resetEmpty(["Alice", "Bob"], 20);
    for (const name of ["Concordant Crossroads", "Celestial Armor", "Grizzly Bears", "Brushland", "Boseiju, Who Endures", "Llanowar Elves", "Sol Ring", "Delighted Halfling", "Plains", "Plains", "Plains", "Plains"]) {
      await game.addCardToZone(0, name, "battlefield", true);
    }

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

test("equipment and simple mana shortcuts", {timeout:120000}, async () => {
 process.env.VITE_E2E_TEST="true";
 const server=await createServer({root,logLevel:"error",optimizeDeps:{entries:["index.html"]},server:{host:"127.0.0.1",port:0,hmr:false,watch:null},plugins:[{
 name:"equip-mana-scenario",transform(code,id) {if(id.endsWith("/context/GameContext.jsx")) return code.replace("const e2eApi = {",scenarioApi);}
 }]});
 await server.listen();
 const browser=await chromium.launch({args:["--disable-webgl"]});
 try {
 const page=await browser.newPage({viewport:{width:1600,height:1000}});
 await page.route("https://fonts.googleapis.com/**", route=>route.fulfill({contentType:"text/css",body:""}));
 await page.addInitScript(()=>localStorage.setItem("ironsmith.showShortcutsOnStart","false"));
 await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/`, {waitUntil:"domcontentloaded",timeout:60000});
 await page.waitForFunction(()=>window.__ironsmithE2E?.snapshot()?.loading===false);
 await page.waitForTimeout(1500);
 await page.evaluate(()=>window.__ironsmithE2E.setupEquipment());
 await page.waitForFunction(()=>window.__ironsmithE2E.scenarioState()?.players?.[0]?.battlefield?.some(c=>c.name==="Celestial Armor"));
 const state=await page.evaluate(()=>window.__ironsmithE2E.scenarioState());
 for (const [name, colors] of [["Brushland",["C"]],["Boseiju, Who Endures",["G"]],["Sol Ring",["C"]],["Llanowar Elves",["G"]],["Delighted Halfling",["C"]]]) {
   const card=state.players[0].battlefield.find(c=>c.name===name);
   const permanent=page.locator(`.battlefield-row-card[data-object-id="${card.id}"]`).first();
   assert.ok(state.decision.actions.some(a=>a.object_id===card.id && a.kind==="activate_mana_ability"), `${name} must have a legal mana activation`);
   await permanent.click();
   const buttons=permanent.locator('.arena-land-mana-strip button');
   await buttons.first().waitFor();
   assert.deepEqual(await buttons.evaluateAll(els=>els.map(el=>el.dataset.manaColor)),colors,name);
   await page.keyboard.press("Escape");
   await page.mouse.click(1590, 400);
 }
 const armor=state.players[0].battlefield.find(c=>c.name==="Celestial Armor");
 await page.locator(`.battlefield-row-card[data-object-id="${armor.id}"]`).first().click();

 const equip=page.getByRole("button",{name:/Celestial Armor: Equip/}).first();
 await equip.waitFor();
 assert.equal(await page.getByRole("button",{name:"Equip rules helper",exact:true}).count(),0);
 await equip.click();
 await page.waitForFunction(()=>window.__ironsmithE2E.scenarioState().decision.kind!=="priority");
 } finally {await browser.close();await server.close();}
});
