// Regenerate the snapshot fixture from the repository root with:
// PREGAME_STACK_EVIDENCE=web/ui/tests/pregame-stack-evidence.json.fixture cargo test -p ironsmith-web-session --lib opening_hand_delayed_stack_text_describes_its_own_effects
import test from 'node:test';
import assert from 'node:assert/strict';
import {chromium} from 'playwright';
import {createServer} from 'vite';
import {readFile, mkdir} from 'node:fs/promises';

test('opening-hand delayed effects appear in the stack frames', async () => {
 const rows=JSON.parse(await readFile(new URL('./pregame-stack-evidence.json.fixture',import.meta.url),'utf8'));
 const vite=await createServer({server:{host:'127.0.0.1',port:0},logLevel:'silent'}); await vite.listen();
 const browser=await chromium.launch();
 try {
  const page=await browser.newPage({viewport:{width:800,height:850}});
  const errors=[]; page.on('pageerror',e=>errors.push(e.message));
  await mkdir('test-results/pregame-stack',{recursive:true});
  for(let i=0;i<rows.length;i++) {
   await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/pregame-stack-text.html?card=${i}`);
   const frame=page.locator('.stack-card--frame'); await frame.waitFor();
   await frame.locator('[data-card-glint-frame="ready"]').waitFor();
   await page.waitForTimeout(1500);
   await page.screenshot({path:`test-results/pregame-stack/${process.env.EVIDENCE_PHASE || 'after'}-${i}.png`});
   if(!process.env.EVIDENCE_PHASE) assert.ok((await frame.locator('canvas').getAttribute('aria-label')).includes(rows[i].expected),await frame.locator('canvas').getAttribute('aria-label'));
  }
  assert.deepEqual(errors,[]);
 } finally {await browser.close();await vite.close();}
});
