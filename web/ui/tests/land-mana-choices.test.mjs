import test from 'node:test';
import assert from 'node:assert/strict';
import {cardSimpleTapManaColors,cardIntrinsicManaColors,cardTypeText,intrinsicManaColors,landManaChoices,manaActionColors,withoutIntrinsicManaLines} from '../src/lib/land-mana-choices.js';
import {cardFrameSnapshot} from '../src/lib/card-frame-snapshot.mjs';

test('intrinsic mana follows basic land types, read from type lines only',()=>{
  assert.deepEqual(intrinsicManaColors('Land — Forest Plains'),['W','G']);
  assert.deepEqual(intrinsicManaColors('Creature — Forest Dryad'),[]);
  const forest={characteristic_signature:'name:Forest\nsupertypes:Basic\ntypes:Land\nsubtypes:Forest\noracle:'};
  assert.deepEqual(intrinsicManaColors(cardTypeText(forest)),['G']);
  // Yavimaya's rules mention Forest; it is not one.
  const yavimaya={characteristic_signature:'name:Yavimaya\ntypes:Land\nsubtypes:\noracle:Each land is a Forest in addition to its other land types.'};
  assert.deepEqual(intrinsicManaColors(cardTypeText(yavimaya)),[]);
});

test('the text box drops one intrinsic line per basic land type and keeps every other ability',()=>{
  assert.deepEqual(withoutIntrinsicManaLines(['{T}: Add {G}.'],['G']),[]);
  assert.deepEqual(withoutIntrinsicManaLines(['I — Gain.','{T}: Add {G}.','{T}: Add {C}.'],intrinsicManaColors('Enchantment Land — Urza’s Saga Forest')),['I — Gain.','{T}: Add {C}.']);
  // A second, equal ability (printed or granted) is a separate line.
  assert.deepEqual(withoutIntrinsicManaLines(['{T}: Add {G}.','{T}: Add {G}.'],['G']),['{T}: Add {G}.']);
  assert.deepEqual(withoutIntrinsicManaLines(['{T}: Add {G}.'],[]),['{T}: Add {G}.']);
  // Live permanents: the engine reports intrinsic mana; their lane says nothing of types.
  const forest={name:'Forest',lane:'lands',intrinsic_mana:['G'],oracle_text:'{T}: Add {G}.'};
  assert.deepEqual(cardIntrinsicManaColors(forest),['G']);
  assert.deepEqual(cardIntrinsicManaColors({name:'Wastes',lane:'lands',oracle_text:'{T}: Add {C}.'}),[]);
  assert.equal(JSON.parse(cardFrameSnapshot(forest)).oracle_text,'');
});

test('each color section maps to the action that makes it, preferring a single-color one',()=>{
  assert.deepEqual(manaActionColors({label:'Activate Savannah: {T}: Add {G} or {W}.'}),['W','G']);
  assert.deepEqual(manaActionColors({label:'Activate Birds: {T}: Add one mana of any color.'}),['W','U','B','R','G']);
  const any={kind:'activate_mana_ability',index:1,label:'Activate X: {T}: Add {G} or {C}.'};
  const colorless={kind:'activate_mana_ability',index:2,label:'Activate X: {T}: Add {C}.'};
  assert.deepEqual(landManaChoices([any,colorless,{kind:'activate_ability',label:'{1}: Add {R}.'}]).map(c=>[c.color,c.action.index]),[['G',1],['C',2]]);
});

test('shortcuts include simple mana on all card types but exclude costs and conditional outputs', () => {
  const action = text => ({kind:'activate_mana_ability',label:`Activate source: ${text}`});
  for (const text of ['{T}: Add {C}.', '{T}: Add {G}.', '{T}: Add {C}{C}.', '{T}: Add one mana of any color.']) {
    assert.ok(landManaChoices([action(text)]).length, text);
  }
  for (const text of [
    '{T}, Pay 1 life: Add {G}.', '{1}, {T}: Add {G}.',
    '{T}, Sacrifice this artifact: Add {G}.',
    '{T}: Add {G} or {W}. This land deals 1 damage to you.',
    '{T}: Add one mana of any color. Spend this mana only to cast a legendary spell.',
    '{T}: Add {G}. Activate only if you control a Forest.',
    '{T}: Add {G} for each creature you control.',
  ]) assert.deepEqual(landManaChoices([action(text)]), [], text);
  const brushland = [action('{T}: Add {C}.'), action('{T}: Add {G} or {W}. This land deals 1 damage to you.')];
  assert.deepEqual(landManaChoices(brushland).map(c=>c.color), ['C']);
  const halfling = [action('{T}: Add {C}.'), action('{T}: Add one mana of any color. Spend this mana only to cast a legendary spell.')];
  assert.deepEqual(landManaChoices(halfling).map(c=>c.color), ['C']);
});


test('nonland passive strips use live simple abilities and survive tapped state',()=>{
  assert.deepEqual(cardSimpleTapManaColors({tapped:true,abilities:['{T}: Add {C}{C}.']}),['C']);
  assert.deepEqual(cardSimpleTapManaColors({abilities:['{T}: Add {C}.','{T}: Add one mana of any color. Spend this mana only to cast a legendary spell.']}),['C']);
  assert.deepEqual(cardSimpleTapManaColors({abilities:[],oracle_text:'{T}: Add {G}.'}),[]);
});
