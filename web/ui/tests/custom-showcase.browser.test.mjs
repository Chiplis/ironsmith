import test from 'node:test';
import assert from 'node:assert/strict';
import {chromium} from 'playwright';
import {createServer} from 'vite';
import {mkdir} from 'node:fs/promises';
import {fileURLToPath} from 'node:url';
const runtimePath=fileURLToPath(new URL('../../wasm_demo/pkg/ironsmith.js',import.meta.url));
test('custom showcase URL reaches a playable real engine table', {timeout:180000}, async()=>{
 const vite=await createServer({define:{'import.meta.env.VITE_E2E_TEST':JSON.stringify('true')},server:{host:'127.0.0.1',port:0,hmr:false},logLevel:'error'});await vite.listen();
 const browser=await chromium.launch({args:['--enable-unsafe-swiftshader']});
 try {
  const page=await browser.newPage({viewport:{width:1440,height:1000}});const errors=[];
  page.on('pageerror',e=>errors.push(e.message));
  page.on('console',m=>{if(m.type()==='error')console.error(m.text());});
  const localImage=page.waitForResponse(response=>response.url().includes('/custom-sets/images/') && response.ok(),{timeout:150000});
  await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/?showcase=custom`,{waitUntil:'domcontentloaded'});
  await page.waitForFunction(()=>document.querySelector('[data-workspace-shell]')||document.body.textContent.includes('Game initialization failed'),null,{timeout:150000});
  const body=await page.locator('body').innerText();
  assert.equal(await page.locator('[data-workspace-shell]').count(),1,body.slice(0,2000));
  assert.ok(body.includes('IronSmith Mana Fixer'),body.slice(0,2000));
  const state=await page.evaluate(()=>window.__ironsmithE2E.snapshot().state);
  assert.equal(state.players.length,2);
  for(const player of state.players) {
    assert.equal(player.hand_size,7);assert.equal(player.library_size,30);
    assert.ok(player.battlefield.some(card=>card.name==='Omniscience'));
    assert.ok(player.battlefield.some(card=>card.name==='IronSmith Mana Fixer'));
  }
  await localImage;
  console.log('showcase snapshot',JSON.stringify(state.decisionDetail));
  await mkdir('test-results/custom-showcase',{recursive:true});await page.screenshot({path:'test-results/custom-showcase/table.png'});
  assert.deepEqual(errors,[]);
 }finally{await browser.close();await vite.close();}
});
test('bundled sets load lazily into the editable workspace with credits', {timeout:90000}, async()=>{
 const vite=await createServer({server:{host:'127.0.0.1',port:0,hmr:false},logLevel:'error'});await vite.listen();
 const browser=await chromium.launch();
 try {
  const page=await browser.newPage({viewport:{width:1440,height:1000}});
  await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/build-workspace.html`);
  const matches=await page.evaluate(async()=>{const {searchBundledCustomCards}=await import('/src/lib/bundled-custom-sets.js');return searchBundledCustomCards('Abhorrent Exhumer');});
  assert.ok(matches.some(row=>row.card.draft.attribution.setCode==='AFM'));
  await page.getByRole('button',{name:'Build',exact:true}).click();
  await page.getByRole('tab',{name:'Sets',exact:true}).click();
  await page.getByLabel('Saved sets',{exact:true}).selectOption({label:'MSEM · Aftermath'});
  await page.getByText('AFM #81 · mattforsyth',{exact:true}).waitFor();
  await page.getByRole('button',{name:'Edit Abhorrent Exhumer',exact:true}).click();
  assert.equal(await page.getByLabel('Card name',{exact:true}).inputValue(),'Abhorrent Exhumer');
  await page.getByLabel('Card name',{exact:true}).fill('Edited Exhumer');
  await page.getByRole('button',{name:'Save to set',exact:true}).click();
  await page.getByText('Saved Edited Exhumer to MSEM · Aftermath.',{exact:true}).waitFor();
  const saved=await page.evaluate(async()=>{
   const {loadCommunityLibrary}=await import('/src/lib/community-library.js');
   return (await loadCommunityLibrary()).sets.find(s=>s.name==='MSEM · Aftermath');
  });
  assert.equal(saved.source.scope,'msem');assert.equal(saved.code,'AFM');
  const card=saved.cards.find(c=>c.draft.faces[0].name==='Edited Exhumer');
  assert.equal(card.draft.attribution.artist,'mattforsyth');assert.equal(card.draft.attribution.collectorNumber,'81');
  assert.ok(card.draft.customMechanics.some(m=>m.keyword==='engorge'));
 }finally {await browser.close();await vite.close();}
});
test('IronSmith Mana Fixer produces all five colors without tapping or paying mana', {timeout:90000}, async()=>{
 const vite=await createServer({server:{host:'127.0.0.1',port:0,hmr:false},logLevel:'error'});await vite.listen();
 const browser=await chromium.launch();
 try {
  const page=await browser.newPage();
  await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/forge-board.html`);
  const result=await page.evaluate(async runtimePath=>{
    const mod=await import(`/@fs${runtimePath}`);await mod.default();
    const {MANA_FIXER_DRAFT}=await import('/src/lib/custom-showcase.js');
    const {normalizeDraftForApi}=await import('/src/lib/custom-card-draft.js');
    const game=new mod.WasmGame();game.resetEmpty(['Alice','Bob'],20);
    const draft=normalizeDraftForApi(MANA_FIXER_DRAFT);const preview=game.previewCustomCard(draft);
    if(!preview.canCreate)throw Error('Fixer did not compile');
    const id=game.createCustomCard({draft,playerIndex:0,zoneName:'battlefield',skipTriggers:true});game.finishPuzzleSetup();
    let state=game.uiState();
    for(let activation=0;activation<2;activation++){
      if(!state.decision?.actions)throw Error(`After activation ${activation}: ${JSON.stringify(state)}`);
      const action=state.decision.actions.find(a=>a.action_ref?.kind==='activate_mana_ability'&&Number(a.action_ref.source)===Number(id));
      if(!action)throw Error(JSON.stringify(state.decision));
      state=game.dispatch({type:'priority_action',action_ref:action.action_ref});
      if(state.decision?.kind==='mana_payment') state=game.dispatch({type:'mana_payment',response:{action:'confirm',plan_id:state.mana_payment.plan_id,request_hash:state.mana_payment.request_hash}});
    }
    return {pool:state.players[0].mana_pool,tapped:state.players[0].battlefield.find(c=>Number(c.id)===Number(id)).tapped};
  },runtimePath);
  console.log('fixer mana result',JSON.stringify(result));assert.equal(result.tapped,false);
  for(const color of ['white','blue','black','red','green'])assert.equal(result.pool[color],2);
 }finally{await browser.close();await vite.close();}
});
