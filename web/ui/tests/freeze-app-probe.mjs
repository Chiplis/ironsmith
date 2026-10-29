import {chromium} from 'playwright';import {createServer} from 'vite';
const scenarioApi=`const e2eApi = {
 scenarioState:()=>state,
 probe:()=>game.uiState(),
 setupFreeze: scenario => runWasmInteraction(async()=>{
 setAutoPassEnabled(false);setHoldRule('always');
 const {prepareCustomCard,installCustomMechanics}=await import('../lib/custom-mechanics.js');
 await game.resetEmpty(['Alice','Bob'],20);
 const add=async(name,text,types=['Artifact'],zoneName='battlefield',customMechanics=[])=>{
 const p=await prepareCustomCard({faces:[{name,manaCost:'{0}',cardTypes:types,power:'2',toughness:'2',oracleText:text}],customMechanics},x=>game.previewCustomCard(x));
 installCustomMechanics(p.draft.customMechanics);
 return game.createCustomCard({draft:p.input,playerIndex:0,zoneName,skipTriggers:true});
 };
 for(const [text,count,types,zone] of scenario.board||[])for(let i=0;i<count;i++)await add('Support '+i+' '+text.slice(0,15),text,types,zone);
 window.freezeObject=Number(await add('Complex Custom Probe',scenario.text,scenario.types||['Artifact'],'battlefield',scenario.mechanics||[{keyword:'plunder',reminder:'Create a Treasure token.'}]));
 for(let seat=0;seat<2;seat++)for(let i=0;i<10;i++)await game.addCardToZone(seat,'Forest','library',true);
 await game.finishPuzzleSetup();let next;
 for(let i=0;i<30;i++){next=await game.uiState();if(/first.main/i.test(next.phase))break;const a=next.decision?.actions?.find(a=>['keep_opening_hand','begin_game','continue_pregame','pass_priority'].includes(a.kind));if(!a)throw Error(JSON.stringify(next.decision));await game.dispatch({type:'priority_action',action_ref:a.action_ref});}
 await finalizeState(game,next);
 }),`;
process.env.VITE_E2E_TEST='true';
const vite=await createServer({logLevel:'error',server:{host:'127.0.0.1',port:0,hmr:false,watch:null},plugins:[{name:'freeze-probe',transform(code,id){if(id.endsWith('/context/GameContext.jsx'))return code.replace('const e2eApi = {',scenarioApi);}}]});await vite.listen();
const browser=await chromium.launch();const page=await browser.newPage({viewport:{width:1600,height:1000}});page.setDefaultTimeout(10000);page.on('pageerror',e=>console.log('PAGEERROR',e.message));
const limit=(p,label,ms=30000)=>Promise.race([p,new Promise((_,reject)=>{const timer=setTimeout(()=>reject(Error(label+' TIMEOUT')),ms);timer.unref();})]);
const treasure=['{T}, Sacrifice this artifact: Add one mana of any color.',15,['Artifact']];
const cases=[
{name:'simple',text:'Whenever this creature attacks, plunder.',types:['Creature']},
{name:'treasures',text:'{8}{U}{U}: Plunder.',board:[treasure]},
{name:'many',text:Array.from({length:12},(_,i)=>`{${i+1}}: Plunder.`).join('\n'),board:[treasure]},
{name:'granted',text:'Artifacts you control have "{T}: Add one mana of any color."\n{10}: Plunder.',board:[['',25,['Artifact']]]},
{name:'variable',text:'{X}, Sacrifice X artifacts: Plunder.',board:[treasure]},
{name:'sacrifices',text:'Sacrifice three artifacts: Plunder.',board:[treasure]},
{name:'targeting',text:'{1}: Tap any number of target creatures.\n{2}: Plunder.',board:[['',20,['Creature']],treasure]},
{name:'mana trigger',text:'Whenever you tap an artifact for mana, plunder.\n{15}: Draw a card.',mechanics:[{keyword:'plunder',reminder:'Add one mana of any color.'}],board:[treasure]},
];
try{await page.addInitScript(()=>localStorage.setItem('ironsmith.showShortcutsOnStart','false'));await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/`,{waitUntil:'domcontentloaded',timeout:45000});await page.waitForFunction(()=>window.__ironsmithE2E?.snapshot()?.loading===false,{},{timeout:45000});await page.waitForTimeout(1500);
for(const c of cases){console.log(c.name,'setup');try{await limit(page.evaluate(c=>window.__ironsmithE2E.setupFreeze(c),c),'setup',45000);await page.waitForTimeout(300);const id=await page.evaluate(()=>window.freezeObject);console.log(c.name,'click',id);await page.locator(`.battlefield-row-card[data-object-id="${id}"]`).first().click({force:true});console.log(c.name,'clicked',await page.locator('.floating-card-preview').evaluateAll(es=>es.map(e=>({visible:e.dataset.visible,id:e.dataset.previewObjectId}))));await page.locator('.floating-card-preview[data-visible="true"]').first().waitFor();await page.waitForTimeout(2000);console.log(c.name,'UI heartbeat',await limit(page.evaluate(()=>performance.now()),'UI',5000));const s=await limit(page.evaluate(()=>window.__ironsmithE2E.probe()),'engine',10000);console.log(c.name,'engine responsive',s.phase);await page.screenshot({path:`../../work/freeze-investigation/${c.name.replaceAll(' ','-')}.png`});}catch(e){console.log(c.name,'FAIL',e.message);console.log('heartbeat after failure',await limit(page.evaluate(()=>({now:performance.now(),body:document.body.innerText.slice(-1200)})),'failure heartbeat',5000).catch(e=>e.message));await page.screenshot({path:'../../work/freeze-investigation/failure.png',timeout:5000}).catch(()=>{});break;}}
}finally{await browser.close();await vite.close();}
