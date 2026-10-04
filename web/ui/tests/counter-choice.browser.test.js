import test from "node:test";
import assert from "node:assert/strict";
import { chromium } from "playwright";
import { createServer } from "vite";
import react from "@vitejs/plugin-react";
import tailwind from "@tailwindcss/vite";
import path from "node:path";
test("actual counter decision controls submit sparse quantities and enforce complete bounds",async()=>{
 const vite=await createServer({configFile:false,root:process.cwd(),cacheDir:path.resolve("node_modules",`.vite-counter-${process.pid}`),plugins:[react(),tailwind()],define:{__IRONSMITH_RUNTIME_VERSION__:JSON.stringify("counter-contract-fixture")},resolve:{alias:{"@":path.resolve("src")}},server:{host:"127.0.0.1",port:0,fs:{allow:[path.resolve("../.."),"/Users/chiplis/ironsmith/web/ui/node_modules"]}},logLevel:"silent"});await vite.listen();const browser=await chromium.launch();
 try{for(const width of [1100,400]){
  const page=await browser.newPage({viewport:{width,height:700}});const errors=[];page.on("pageerror",e=>errors.push(e.message));await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/counter-choice.html`);
  const charge=page.getByRole("textbox",{name:"Charge counters",exact:true});const plus=page.getByRole("textbox",{name:"+1/+1 counters",exact:true});await charge.waitFor();
  await plus.fill("");await plus.fill("4294967295");await charge.fill("");await charge.fill("4294967295");await page.getByRole("button",{name:"Submit",exact:true}).click();assert.deepEqual(await page.evaluate(()=>window.__commands),[{type:"select_counters",allocations:[{index:1,count:4294967295},{index:0,count:4294967295}]}]);
  await page.evaluate(()=>window.__setDecision({min_total:"3",max_total:"5"}));await page.waitForFunction(()=>document.querySelector('input[aria-label="Charge counters"]')?.value==="3");await charge.fill("2");assert.equal(await page.getByRole("button",{name:"Submit",exact:true}).isDisabled(),true);await charge.fill("6");assert.equal(await page.getByRole("button",{name:"Submit",exact:true}).isDisabled(),true);await charge.fill("3");assert.equal(await page.getByRole("button",{name:"Submit",exact:true}).isDisabled(),false);
  await charge.fill("4294967296");assert.equal(await page.getByRole("button",{name:"Submit",exact:true}).isDisabled(),true);
  await page.evaluate(()=>window.__setDecision({min_total:"8589934590",max_total:"8589934590"}));await page.waitForFunction(()=>[...document.querySelectorAll("input")].every(node=>node.value==="4294967295"));assert.equal(await page.getByRole("button",{name:"Submit",exact:true}).isDisabled(),false);assert.deepEqual(errors,[]);await page.close();
 }
 const page=await browser.newPage();await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/counter-choice.html?spectator`);await page.getByRole("textbox",{name:"Charge counters",exact:true}).waitFor();assert.equal(await page.getByRole("textbox",{name:"Charge counters",exact:true}).isDisabled(),true);assert.equal(await page.getByRole("button",{name:"Submit",exact:true}).isDisabled(),true);await page.close();
 }finally{await browser.close();await vite.close();}
});
