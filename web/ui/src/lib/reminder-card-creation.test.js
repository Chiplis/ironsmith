import {expandNamedActionReplacementReferences} from './reminder-named-action-programs.js';
import test from 'node:test';
import assert from 'node:assert/strict';
import {expandCardCreationPrograms,suppliedCardDefinitions} from './reminder-card-creation.js';
import {compileDraftInput,validateMechanic,draftMechanics} from './custom-mechanics.js';
import {normalizeDraftForApi} from './custom-card-draft.js';
const definition=validateMechanic({keyword:'Conjure',declarationKind:'card-creation',reminder:'Create real cards from supplied definitions in the declared zone.'});
const pool=['Fatigue','Mountain','Plains','Swamp','Virtual Avatar'].map(name=>({name,text:'Type: Sorcery\nDraw a card.'}));
const expand=text=>expandCardCreationPrograms(text,[definition],pool);
test('supplied custom identity remains explicit and creation is real-card data',()=>{
 const source='Conjure a card named Fatigue into your hand.';
 const draft={suppliedCardDefinitions:pool,faces:[{name:'Caller',oracleText:source}]};
 assert.equal(draftMechanics(draft,[definition]).length,1);
 const output=compileDraftInput(draft,[definition]);const face=output.faces[0];
 assert.equal(face.oracleText,'Create a real card named "Fatigue" in your hand as part of the named conjure action.');
 assert.ok(!face.oracleText.includes('token'));
 assert.deepEqual(face.customKeywordIdentities||[],[]);
 assert.equal(face.suppliedCardDefinitions[0].text,'Type: Sorcery\nDraw a card.');
 const normalized=normalizeDraftForApi(output);assert.equal(normalized.suppliedCardDefinitions.length,5);assert.equal(normalized.faces[0].suppliedCardDefinitions.length,5);
});
test('one multi-identity batch differs from a resolution-time identity choice',()=>{
 const batch=expand('Conjure a card named Mountain, a card named Plains, and a card named Swamp into your hand.');
 assert.ok(batch.includes('Create a real card for each name "Mountain", "Plains", "Swamp"'));
 assert.equal(batch.split('named conjure action').length-1,1);
 const choice=expand('Conjure a card named Mountain, a card named Plains, or a card named Swamp into your hand.');
 assert.ok(choice.startsWith('Choose one of the following: ['));assert.equal(choice.split('] or [').length,3);
});
test('library placement is creation destination and shuffle is a sibling instruction',()=>{
 assert.ok(expand('Conjure three cards named Fatigue and put them on top of your library.').includes('Create three real cards named "Fatigue" on top of your library'));
 assert.ok(expand('Conjure ten cards named Fatigue and shuffle them into your library.').endsWith('; Shuffle your library.'));
 assert.ok(expand('Conjure a card named Virtual Avatar onto the battlefield.').includes('on the battlefield'));
});
test('optional each-player operation scopes the whole iteration',()=>{
 const output=expand('You may have each player conjure a card named Fatigue into their hand.');
 assert.ok(output.startsWith('You may do the following: [For each player, do the following: ['));
 assert.ok(output.includes("in that player's hand"));assert.ok(output.endsWith(']].'));
});
test('observers and same-name history consume ordinary exact action receipts',()=>{
 assert.equal(expand('Whenever you conjure one or more cards, you gain that much life.'),'Whenever one or more cards participate in the conjure action performed by you, you gain that much life.');
 assert.equal(expand('Whenever you conjure a land card into your hand, you gain 1 life.'),'Whenever a land card in your hand participates in the conjure action performed by you, you gain 1 life.');
 assert.ok(!expand('Whenever you conjure one or more cards, you gain that much life.').includes('with an amount of'),'the observer requires actual matching created objects, not an amount-only completion');
 assert.equal(expand("Spell you cast with the same name as a card you've conjured this turn cost {1} less to cast."),'Spells you cast sharing a name with a card from your conjure action this turn cost {1} less to cast.');
 assert.ok(expand('If a player would conjure a card, they instead conjure a card named Fatigue.').includes('use the supplied card definition named "Fatigue" instead'));
});
test('one random batch and repeated random operations retain different receipt cardinality',()=>{
 const batch=expand('Conjure three of a random creature card into your hand.');assert.ok(batch.startsWith('Create three real cards matching randomly creature card'));
 const repeated=expand('Conjure a random creature card into your hand three times.');assert.ok(repeated.startsWith('Repeat the following process three times: [Create a real cards matching randomly creature card'));
 assert.ok(expand('Conjure a duplicate of target creature into your hand.').includes('duplicating the target creature'));
});
test('declared operation spelling is data and quoted card names stay opaque',()=>{
 const custom=validateMechanic({keyword:'Call',declarationKind:'card-creation',reminder:'Create real cards from supplied definitions in the declared zone.'});
 assert.ok(expandCardCreationPrograms('Call two cards named Fatigue into your hand.',[custom],pool).includes('named call action'));
 const quoted='Name a card "Conjure a card named Fatigue into your hand."';assert.equal(expand(quoted),quoted);
 const gained='This permanent gains “Once each turn, when you sacrifice a permanent, you may conjure a card named Fatigue into your hand.”';
 assert.ok(expand(gained).includes("named 'Fatigue'"));
});
test('conflicting supplied definitions fail while object-key order does not change identity',()=>{
 assert.equal(suppliedCardDefinitions({suppliedCardDefinitions:[{name:'Fatigue',text:'Draw a card.'},{text:'Draw a card.',name:'Fatigue'}]}).length,1);
 assert.throws(()=>suppliedCardDefinitions({suppliedCardDefinitions:[{name:'Fatigue',text:'Draw a card.'},{name:'Fatigue',text:'You gain 2 life.'}]}),/Conflicting/);
 assert.throws(()=>suppliedCardDefinitions({suppliedCardDefinitions:[{name:'Fatigue',text:'Draw a card.',token:true}]}),/real cards/);
});

test('proposed creation uses its dedicated supplied-identity contract, not an object operand template',()=>{
 const declared=validateMechanic({keyword:'Conjure',declarationKind:'card-creation',reminder:'Create real cards from supplied definitions in the declared zone.'});
 const source='If a player would conjure a card, they instead conjure a card named Data-Point.';
 const expected='If a player would create real cards as part of the named conjure action, use the supplied card definition named "Data-Point" instead.';
 for(const definitions of [[],[{name:'Data-Point',text:'Mana cost: {2}\nType: Sorcery\nDraw a card.'}]]){
  const creation=expandCardCreationPrograms(source,[declared],definitions);
  assert.equal(creation,expected);
  assert.equal(expandNamedActionReplacementReferences(creation,[declared]),expected);
  assert.ok(!creation.includes('perform the conjure action with one or more cards'));
  assert.deepEqual(definitions,definitions.length?[{name:'Data-Point',text:'Mana cost: {2}\nType: Sorcery\nDraw a card.'}]:[]);
 }
 assert.throws(()=>validateMechanic({keyword:'Conjure',declarationKind:'card-creation',template:'{Object}',reminder:'Create real cards from supplied definitions in the declared zone.'}),/without an operand template/);
 assert.throws(()=>expandNamedActionReplacementReferences('If a player would invent a card, draw a card.',[validateMechanic({keyword:'Invent',reminder:'Draw a card.'})]),/explicit collective object action program/);
});
