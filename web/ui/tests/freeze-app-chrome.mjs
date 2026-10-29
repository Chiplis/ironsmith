import {writeFile} from 'node:fs/promises';
import {chromium} from 'playwright';import {createServer} from 'vite';
const scenarioApi=`const e2eApi = {
 scenarioState:()=>state,
 probe:()=>game.uiState(),
 inspectorProbe:()=>game.inspectorActions(BigInt(window.freezeObject)),
 setupFreeze: scenario => runWasmInteraction(async()=>{
 setAutoPassEnabled(false);setHoldRule('always');
 const {prepareCustomCard,installCustomMechanics}=await import('../lib/custom-mechanics.js');
 await game.resetEmpty(['Alice','Bob'],20);
 const add=async(name,text,types=['Artifact'],zoneName='battlefield',customMechanics=[])=>{
 const p=await prepareCustomCard({faces:[{name,manaCost:'{0}',cardTypes:types,power:'2',toughness:'2',oracleText:text}],customMechanics},x=>game.previewCustomCard(x));
 installCustomMechanics(p.draft.customMechanics);
 return game.createCustomCard({draft:p.input,playerIndex:0,zoneName,skipTriggers:true});
 };
 let group=0;for(const [text,count,types,zone] of scenario.board||[]){const name='Support Group '+(++group);await add(name,text,types,zone);for(let i=1;i<count;i++)await game.addCardToZone(0,name,zone||'battlefield',true);}
 window.freezeObject=Number(await add('Complex Custom Probe',scenario.text,scenario.types||['Artifact'],'battlefield',scenario.mechanics||[{keyword:'plunder',reminder:'Create a Treasure token.'}]));
 for(let seat=0;seat<2;seat++)for(let i=0;i<10;i++)await game.addCardToZone(seat,'Forest','library',true);
 await game.finishPuzzleSetup();let next;
 for(let i=0;i<30;i++){next=await game.uiState();if(/first.main/i.test(next.phase))break;const a=next.decision?.actions?.find(a=>['keep_opening_hand','begin_game','continue_pregame','pass_priority'].includes(a.kind));if(!a)throw Error(JSON.stringify(next.decision));await game.dispatch({type:'priority_action',action_ref:a.action_ref});}
 await finalizeState(game,next);
 }),`;
process.env.VITE_E2E_TEST='true';
const vite=await createServer({logLevel:'error',server:{host:'127.0.0.1',port:0,hmr:false,watch:null},plugins:[{name:'freeze-probe',transform(code,id){if(id.endsWith('/context/GameContext.jsx'))return code.replace('const e2eApi = {',scenarioApi);}}]});await vite.listen();
const browser=await chromium.launch({channel:"chrome"});const page=await browser.newPage({viewport:{width:1600,height:1000}});page.setDefaultTimeout(10000);page.on('pageerror',e=>console.log('PAGEERROR',e.message));
const cdp=await page.context().newCDPSession(page);await cdp.send('Profiler.enable');await cdp.send('Profiler.start');
const limit=(p,label,ms=30000)=>Promise.race([p,new Promise((_,reject)=>{const timer=setTimeout(()=>reject(Error(label+' TIMEOUT')),ms);timer.unref();})]);
const treasure=['{T}, Sacrifice this artifact: Add one mana of any color.',15,['Artifact']];
const cases=[
{name:'converters',text:'{30}: Plunder.',board:[['{1}, {T}, Sacrifice another artifact: Add three mana of any one color.',12,['Artifact']],['{T}: Add {C}.',5,['Artifact']]]},
{name:'large treasures',text:'{63}{U}: Plunder.',board:[['{T}, Sacrifice this artifact: Add one mana of any color.',64,['Artifact']]]},
{name:'borrowed',text:'This creature has all activated abilities of other creatures you control.\n{10}: Plunder.',types:['Creature'],board:[['{1}: Untap target creature.',8,['Creature']],['{T}: Add {G}.',8,['Creature']]]},
{name:'many graves',text:'{2}, Exile ten creature cards from your graveyard: Plunder.',board:[['',40,['Creature'],'graveyard'],['{T}: Add {C}.',4,['Artifact']]]},
{name:'mana replacement',text:'{1000}: Plunder.',board:[['If you tap a permanent for mana, it produces twice as much of that mana instead.',8,['Enchantment']],['{T}: Add one mana of any color.',8,['Artifact']]]},
{name:'tokens and mana',text:'{25}: Plunder.',board:[['Sacrifice a creature: Add one mana of any color.',3,['Artifact']],['',25,['Creature']]]},

];
try{await page.addInitScript(()=>{localStorage.setItem('ironsmith.showShortcutsOnStart','false');localStorage.setItem('ironsmith.fixedStartingBoard','true');});await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/`,{waitUntil:'domcontentloaded',timeout:45000});await page.waitForFunction(()=>window.__ironsmithE2E?.snapshot()?.loading===false,{},{timeout:45000});await page.waitForTimeout(1500);
for(const c of cases){console.log(c.name,'setup');try{await limit(page.evaluate(c=>window.__ironsmithE2E.setupFreeze(c),c),'setup',90000);await page.waitForTimeout(300);const id=await page.evaluate(()=>window.freezeObject);console.log(c.name,'click',id,await page.evaluate(()=>({cards:window.__ironsmithE2E.scenarioState().players[0].battlefield.map(c=>[c.id,c.name]),dom:[...document.querySelectorAll('.battlefield-row-card')].map(e=>[e.dataset.objectId,e.dataset.cardName])})));const box=await page.locator(`.battlefield-row-card[data-object-id="${id}"]`).first().boundingBox();await page.mouse.click(box.x+box.width/2,box.y+box.height/2);console.log(c.name,'clicked',await page.locator('.floating-card-preview').evaluateAll(es=>es.map(e=>({visible:e.dataset.visible,id:e.dataset.previewObjectId}))));await page.locator('.floating-card-preview[data-visible="true"]').first().waitFor();await page.waitForTimeout(2000);console.log(c.name,'UI heartbeat',await limit(page.evaluate(()=>performance.now()),'UI',5000));const s=await limit(page.evaluate(()=>window.__ironsmithE2E.probe()),'engine',10000);console.log(c.name,'engine responsive',s.phase);const actions=await limit(page.evaluate(()=>window.__ironsmithE2E.inspectorProbe()),'inspector',20000);console.log(c.name,'inspector settled',actions.length);await page.screenshot({path:`../../work/freeze-investigation/${c.name.replaceAll(' ','-')}.png`});}catch(e){console.log(c.name,'FAIL',e.message);await writeFile('../../work/freeze-investigation/freeze.cpuprofile',JSON.stringify((await cdp.send('Profiler.stop')).profile));console.log('CPU profile saved');console.log('heartbeat after failure',await limit(page.evaluate(()=>({now:performance.now(),body:document.body.innerText.slice(-1200)})),'failure heartbeat',5000).catch(e=>e.message));await writeFile('../../work/freeze-investigation/stress-diagnostics.json',JSON.stringify(await limit(page.evaluate(async()=>{const d=await import('/src/lib/action-diagnostics.js');return d.exportDiagnostics(null,window.__ironsmithE2E.scenarioState());}),'diagnostics',5000).catch(e=>({error:e.message})),null,2));await page.screenshot({path:'../../work/freeze-investigation/failure.png',timeout:5000}).catch(()=>{});break;}}
}finally{await browser.close();await vite.close();}
