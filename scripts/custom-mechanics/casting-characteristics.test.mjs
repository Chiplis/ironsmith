import test from 'node:test';
import assert from 'node:assert/strict';
import {compileDraftInput,validateMechanic} from '../../web/ui/src/lib/custom-mechanics.js';
import {expandPrintedCastingCharacteristics} from '../../web/ui/src/lib/reminder-casting-characteristics.js';
const reminder='As a sorcery, you may pay {2} and exile this card from your hand face down. Cast it on a later turn for its printed alternative mana cost. When cast this way, this spell and the permanent it becomes have the printed alternative mana cost, its corresponding colors, and the printed alternative power and toughness if present. It keeps its other characteristics. These are copiable characteristics.';
test('arbitrary named declarations bind mana/color and optional size independently',()=>{
 const def=validateMechanic({keyword:'Future form',reminder});
 for(const [declaration,pt] of [['Future form {2}{B} — 3/2',', and base power and toughness 3/2'],['Future form {1}{R}{R}','']]){
  const face=compileDraftInput({faces:[{name:'Witness',oracleText:declaration+'\nFlying'}]},[def]).faces[0];
  assert.match(face.oracleText,/As a sorcery, you may pay \{2\}/);
  assert.ok(face.oracleText.includes('colors determined by that mana cost'+pt+'.'));
  assert.ok(face.oracleText.endsWith('\nFlying'));
  assert.deepEqual(face.customKeywordIdentities,['future form']);
  assert.deepEqual(face.customKeywordPayments,['future form']);
 }
});
test('selected resolution instructions bind only to their source and permission',()=>{
 const def=validateMechanic({keyword:'Future form',reminder});
 const text='Future form {4}{B}{B} — 7/7\nDraw a card.\nIf Witness was cast for its future form cost, it enters as a Nightmare creature after it resolves.';
 const face=compileDraftInput({faces:[{name:'Witness',oracleText:text}]},[def]).faces[0];
 assert.match(face.oracleText,/When cast this way, after resolving this spell enters the battlefield as a Nightmare creature/);
 assert.equal(face.displayOracleText,text);
 assert.ok(face.oracleText.indexOf('Nightmare')<face.oracleText.indexOf('Draw a card'));
 const foreign=expandPrintedCastingCharacteristics(text.replace('If Witness','If Other witness'),def,'Witness');
 assert.match(foreign,/If Other witness was cast for its future form cost/);
});
test('quoted references and incomplete alternative parameters do not expand',()=>{
 const def=validateMechanic({keyword:'Future form',reminder});
 for(const text of ['"Future form {B} — 2/2"','Future form {B} — */*','Future form {B} — 2/','Future form']){
  assert.equal(expandPrintedCastingCharacteristics(text,def,'Witness'),text);
 }
});
