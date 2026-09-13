import test from 'node:test';
import assert from 'node:assert/strict';
import {chromium} from 'playwright';
import {createServer} from 'vite';

test('leaving the Main 2 button clears its glow before a busy snapshot finishes rendering',async()=>{
  const vite=await createServer({server:{host:'127.0.0.1',port:0},logLevel:'silent'});
  await vite.listen();const browser=await chromium.launch();
  try {
    const page=await browser.newPage();
    const errors=[];page.on('pageerror',error=>errors.push(error.message));
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/decision-hover.html?stress`);
    const button=page.locator('.action-strip-advance-button');
    await button.waitFor();
    const title=page.locator('.action-strip-main-title-text');
    const idleShadow=await title.evaluate(el=>getComputedStyle(el).textShadow);
    await button.hover();
    assert.notEqual(await title.evaluate(el=>getComputedStyle(el).textShadow),idleShadow);
    await button.evaluate(el=>el.addEventListener('pointerleave',()=>{
      window.__leftAt=performance.now();
      window.__committedAtLeave=Boolean(window.__phaseCommittedAt);
      window.__shadowAtLeave=getComputedStyle(document.querySelector('.action-strip-main-title-text')).textShadow;
    }));
    await page.mouse.down();await page.mouse.up();
    await page.waitForFunction(()=>window.__snapshotQueuedAt);
    assert.equal(await page.evaluate(()=>window.__authoritativePhase),'NextMain');
    await page.mouse.move(900,700);
    await page.waitForFunction(()=>window.__phaseCommittedAt);
    const result=await page.evaluate(()=>({left:window.__leftAt,committedAtLeave:window.__committedAtLeave,shadow:window.__shadowAtLeave,committed:window.__phaseCommittedAt}));
    assert.equal(result.committedAtLeave,false,JSON.stringify(result));
    assert.equal(result.shadow,idleShadow,'hover glow clears before the board commit');
    assert.ok(result.left<result.committed);
    assert.equal(await page.locator('output').textContent(),'NextMain');
    const ordering=await page.evaluate(()=>{
      window.__publishSnapshot(previous=>({...previous,snapshot_id:3,phase:'Beginning'}));
      window.__publishSnapshot(previous=>({...previous,snapshot_id:previous.snapshot_id+1,phase:'Ending'}));
      return {authoritative:window.__readSnapshot().snapshot_id,sequencer:window.__sequencerSnapshot.snapshot_id,ready:window.__isSnapshotRendered()};
    });
    assert.deepEqual(ordering,{authoritative:4,sequencer:4,ready:false},'command paths receive the new snapshot but old controls cannot submit against it');
    await page.waitForFunction(()=>document.querySelector('output').textContent==='Ending');
    assert.equal(await page.evaluate(()=>window.__readSnapshot().snapshot_id),4,'render commits never restore an older authoritative state');
    assert.equal(await page.evaluate(()=>window.__isSnapshotRendered()),true);
    assert.deepEqual(errors,[]);
  } finally {await browser.close();await vite.close();}
});
