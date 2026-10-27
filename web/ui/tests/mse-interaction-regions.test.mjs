import test from 'node:test';
import assert from 'node:assert/strict';
import {mseInteractionRegions} from '../src/lib/mse/interaction-regions.mjs';

function result(text,{angle=0,split=text.length,stretch=1}={}){
  const line=(start,end,top)=>({start,end,top,height:12,positions:Array.from({length:end-start+1},(_,i)=>i*5)});
  return {width:375,height:523,context:{fieldVisible:()=>true,property:(_name,key)=>({left:20,top:300,height:100,angle}[key]),
    textMeasurements:new Map([['card.rule_text',{result:{stretch,measurement:{runs:[{text,characterStart:0}]},
      lines:split===text.length?[line(0,text.length,0)]:[line(0,split,0),line(split,text.length,12)]}}]])}};
}
test('ability targets follow measured wrapping and field rotation',()=>{
  const action={ability_index:2},view={lines:['{T}: Draw a card.'],actions:new Map([[0,[action]]])};
  const regions=mseInteractionRegions(result('T: Draw a card.',{split:8,angle:90}),view);
  assert.equal(regions.length,2);
  assert.equal(regions[0].actions[0],action);
  assert.equal(regions[0].angle,-90);
  assert.equal(regions[1].left,32);
  assert.equal(regions[1].top,300);
  assert.ok(regions.every(r=>r.height===12));
});
test('grouped mana symbols have distinct measured hit targets and actions',()=>{
  const white={ability_index:0},blue={ability_index:1};
  const view={lines:['{T}: Add {W} or {U}.'],actions:new Map([[0,[white,blue]]]),manaGroups:new Map([[0,{
    prefix:'{T}: Add ',suffix:'.',parts:[{type:'element',value:'{W}'},{type:'literal',value:' or '},{type:'element',value:'{U}'}],
    options:[{output:'{W}',actions:[white]},{output:'{U}',actions:[blue]}],
  }]])};
  const regions=mseInteractionRegions(result('T: Add W or U.'),view);
  assert.equal(regions.length,2);
  assert.equal(regions[0].actions[0],white);assert.equal(regions[1].actions[0],blue);
  assert.equal(regions[0].width,5);assert.ok(regions[1].left>regions[0].left);
});
test('unmatched text never gets a guessed click region',()=>{
  assert.deepEqual(mseInteractionRegions(result('Different ability'),{lines:['Draw a card.'],actions:new Map([[0,[{}]]])}),[]);
});

test('rules helpers resolve exact substrings without matching card names',()=>{
  const fixture=result('Flying');
  fixture.context.textMeasurements=new Map([['card.name',{result:{measurement:{runs:[{text:'Flying Men',characterStart:0}]},lines:[{start:0,end:10,top:0,height:10,positions:Array.from({length:11},(_,i)=>i*3)}]}}],...fixture.context.textMeasurements]);
  const helper={type:'keyword',text:'Flying'};
  const regions=mseInteractionRegions(fixture,{lines:['Flying'],actions:new Map(),ranges:new Map([[0,[{start:0,end:6,helper}]]])});
  assert.equal(regions.length,1);assert.equal(regions[0].width,30);assert.equal(regions[0].helper,helper);
});


test('Equip activation owns its keyword and cost hit area; inactive keywords keep help', async()=>{
  const {mseRulesHelperView}=await import('../src/lib/mse/rules-helper-ranges.mjs');
  const view={lines:['Flash','Equip {3}{W}'],actions:new Map([[1,[{kind:'activate_ability'}]]])};
  const helpers=mseRulesHelperView(view);
  assert.ok(helpers.ranges.get(0).some(range=>range.helper.type==='keyword'));
  assert.deepEqual(helpers.ranges.get(1),[]);
  assert.ok(mseRulesHelperView({...view,actions:new Map()}).ranges.get(1).length);
});
