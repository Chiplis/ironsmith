import test from 'node:test';
import assert from 'node:assert/strict';
import {createServer} from 'vite';
import {chromium} from 'playwright';
import {fileURLToPath} from 'node:url';
const runtimePath=fileURLToPath(new URL('../../wasm_demo/pkg/ironsmith.js',import.meta.url));
test('real imported Improve and Advance actions map to their printed and compiled rows',{timeout:90000},async()=>{
 const vite=await createServer({server:{host:'127.0.0.1',port:0,hmr:false},logLevel:'error'});await vite.listen();const browser=await chromium.launch();
 try{const page=await browser.newPage();await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/keyword-activation-rows.html`);
 const results=await page.evaluate(async path=>{
  const wasm=await import('/@fs'+path);await wasm.default();
  const {prepareCustomCard}=await import('/src/lib/custom-mechanics.js');
  const {normalizeDraftForApi}=await import('/src/lib/custom-card-draft.js');
  const {interactiveRulesView,inspectorRulesLines,activatedAbilityLineIndices}=await import('/src/lib/inspector-ability-lines.js');
  const results=[];
  for(const [code,name] of [['MSY','Herald of Tedium'],['KXP','Shearing Vampire']]){
   const set=await (await fetch(`/custom-sets/msem-${code}.json`)).json();const card=set.cards.find(c=>c.draft.faces[0].name===name);
   const game=new wasm.WasmGame();game.registerExternalCardSources(JSON.parse(game.getEmbeddedCardSourceJson('swamp')));game.resetEmpty(['Alice','Bob'],20);
   const {input}=await prepareCustomCard(normalizeDraftForApi(card.draft),draft=>game.previewCustomCard(draft));
   const id=game.createCustomCard({draft:input,playerIndex:0,zoneName:'battlefield',skipTriggers:true});
   for(let i=0;i<5;i++)game.addCardToZone(0,'Swamp','battlefield',true);
   game.finishPuzzleSetup();let state=game.uiState();
   for(let i=0;i<30&&state.phase!=='first main phase';i++){
    const a=state.decision?.actions?.find(a=>['keep_opening_hand','begin_game','continue_pregame','pass_priority'].includes(a.kind));
    if(!a)throw Error(JSON.stringify(state.decision));state=game.dispatch({type:'priority_action',action_ref:a.action_ref});
   }
   const actions=state.decision?.actions?.filter(a=>a.kind==='activate_ability'&&Number(a.object_id??a.action_ref?.source)===Number(id))||[];
   const details=game.objectDetails(id);const printed=card.draft.faces[0].oracleText;
   const compiled=inspectorRulesLines(details.compiled_text,details.abilities).join('\n');
   const printView=interactiveRulesView(printed,printed,actions,input.customMechanics);
   const compiledView=interactiveRulesView(compiled,compiled,actions,input.customMechanics);
   results.push({name,actions,printedRows:[...printView.actions.keys()],compiledRows:[...compiledView.actions.keys()],compiledLines:compiledView.lines,activatedRows:activatedAbilityLineIndices(compiledView.lines,input.customMechanics)});
   game.free();
  }
  return results;
 },runtimePath);
 for(const result of results){assert.ok(result.actions.length,JSON.stringify(result));assert.deepEqual(result.printedRows,[0],JSON.stringify(result));assert.equal(result.compiledRows.length,1,JSON.stringify(result));assert.ok(result.activatedRows.includes(result.compiledRows[0]),JSON.stringify(result));}
 }finally{await browser.close();await vite.close();}
});
