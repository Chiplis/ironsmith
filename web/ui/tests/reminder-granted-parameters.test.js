import test from 'node:test';
import assert from 'node:assert/strict';
import { validateMechanic,draftMechanics,compileDraftInput } from '../src/lib/custom-mechanics.js';
import { expandGrantedParameterizedAbilities } from '../src/lib/reminder-granted-parameters.js';
const reminder='{Cost}: Put a +1/+1 counter on this permanent. This ability costs {1} more to activate for each +1/+1 counter on this permanent. Activate only as a sorcery.';
test('a final parameter activation grant retains other abilities, identity and whole program for arbitrary names',()=>{
  for(const keyword of ['Improve','Steady craft']) {
    const def=validateMechanic({keyword:`${keyword} {Cost}`,reminder});
    const original=`Equipped creature has hexproof and ${keyword} {1}.`;
    const result=expandGrantedParameterizedAbilities(original,[def]);
    assert.ok(result.startsWith('Equipped creature has hexproof, custom keyword '));
    assert.ok(result.includes(`custom keyword "${keyword.toLowerCase()}"`));
    assert.ok(result.includes('"{1}: Put a +1/+1 counter'));
    assert.ok(result.includes('This ability costs {1} more to activate for each +1/+1 counter'));
    assert.ok(result.includes('Activate only as a sorcery."'));
  }
});
test('non-grants, quoted names, incomplete/control-flow declarations and nonactivation definitions stay unchanged',()=>{
  const def=validateMechanic({keyword:'Improve {Cost}',reminder});
  for(const text of ['Improve {1}.','A card named "Equipped creature has Improve {1}."','Equipped creature has Improve {1} if you attacked this turn.','Equipped creature has Improve {1} and flying.'])assert.equal(expandGrantedParameterizedAbilities(text,[def]),text);
  const entry=validateMechanic({keyword:'Gather {Cost}',reminder:'You may pay {Cost} rather than pay this spell\'s mana cost.'});
  assert.equal(expandGrantedParameterizedAbilities('Equipped creature has Gather {1}.',[entry]),'Equipped creature has Gather {1}.');
  assert.throws(()=>expandGrantedParameterizedAbilities('Equipped creature has Flying {1}.',[{keyword:'flying',template:'{Cost}',reminder,expansion:reminder,version:1}]),/validated custom/);
});
test('draft integration preserves display and assigns identity to recipient rather than grantor',()=>{
  const def=validateMechanic({keyword:'Improve {Cost}',reminder});const original='Equipped creature has hexproof and Improve {1}.';
  const face=compileDraftInput({faces:[{name:'Teaching Equipment',oracleText:original}]},[def]).faces[0];
  assert.equal(face.displayOracleText,original);assert.match(face.oracleText,/custom keyword "improve"/);
  assert.equal(face.customKeywordIdentities,undefined);assert.equal(face.customKeywordPayments,undefined);
});

test('draftMechanics retains known and supplied Equipment-only grant definitions before compilation',()=>{
  const def=validateMechanic({keyword:'Improve {Cost}',reminder});
  const original='Equipped creature has hexproof and improve {1}.\n{4}: Put Faithforged Pauldron from your hand onto the battlefield attached to target creature.\nEquip {1}';
  for(const supplied of [false,true]) {
    const draft={faces:[{name:'Faithforged Pauldron',oracleText:original}],customMechanics:supplied?[def]:[]};
    const definitions=draftMechanics(draft,supplied?[]:[def]);
    assert.equal(definitions.length,1);
    const face=compileDraftInput(draft,definitions).faces[0];
    assert.equal(face.displayOracleText,original);
    assert.match(face.oracleText,/custom keyword "improve"/);
    assert.ok(face.oracleText.includes('This ability costs {1} more to activate'));
    assert.ok(face.oracleText.includes('{4}: Put Faithforged Pauldron'));
    assert.equal(face.customKeywordIdentities,undefined);
  }
  assert.deepEqual(draftMechanics({faces:[{name:'Other',oracleText:'Equipped creature has flying.'}]},[def]),[]);
});
test('numeric token grants split independent entry and triggered abilities without marking the creator',()=>{
 const def=validateMechanic({keyword:'Burden {Number}',reminder:'This permanent enters with {Number} -1/-1 counters on it. Whenever this creature deals combat damage to a player, remove all -1/-1 counters from it.'});
 const original='Create three 2/2 red Noble creature tokens with burden 1.';
 const face=compileDraftInput({faces:[{name:'Creator',oracleText:original}]},[def]).faces[0];
 assert.equal(face.oracleText,'Create three 2/2 red Noble creature tokens with custom keyword "burden", and "This permanent enters with 1 -1/-1 counters on it.", and "Whenever this creature deals combat damage to a player, remove all -1/-1 counters from it.".');
 assert.equal(face.customKeywordIdentities,undefined);assert.equal(face.displayOracleText,original);
});
test('numeric recipient power expressions stay inside the granted program',()=>{
 const def=validateMechanic({keyword:'Mirage {Number}',reminder:"At the beginning of combat on your turn, you may tap any number of other creatures you control with total power {Number} or greater. If you do, create a token that's a copy of this permanent, except it has haste. Sacrifice the token at the beginning of the next end step."});
 const original='Nontoken creatures you control have mirage X, where X is their power.';
 const face=compileDraftInput({faces:[{name:'Grantor',oracleText:original}]},[def]).faces[0];
 assert.match(face.oracleText,/total power X or greater, where X is this creature's power\./);assert.equal(face.customKeywordIdentities,undefined);assert.equal(face.displayOracleText,original);
});
test('explicit mana operands compose with additional activation payments on graveyard card recipients',()=>{
 const def=validateMechanic({keyword:'Renew',reminder:"{Cost}, Exile this card from your graveyard: Create a token that's a copy of this card, except it has no mana cost. Activate only as a sorcery."});
 const original='Creature and Aura cards with mana value 3 or less in your graveyard have renew {2}{W/B}.';
 const face=compileDraftInput({faces:[{name:'Graveyard teacher',oracleText:original}]},[def]).faces[0];assert.ok(face.oracleText.includes('"{2}{W/B}, Exile this card from your graveyard:'));assert.equal(face.customKeywordIdentities,undefined);
});
test('explicit historical price clauses bind the recipient mana cost and additional fixed price',()=>{
 const def=validateMechanic({keyword:'Renew',reminder:"{Cost}, Exile this card from your graveyard: Create a token that's a copy of this card, except it has no mana cost. Activate only as a sorcery."});
 for(const [original,cost] of [
 ['Up to two target creature cards in your graveyard gain renew until end of turn. Their renew costs are equal to their mana cost.',"Pay this card's mana cost"],
 ['Whenever a creature card is put into your graveyard from anywhere, it gains renew until the beginning of your next end step. The renew cost is equal to its mana cost increased by {1}.',"Pay this card's mana cost, {1}"]]){
 const face=compileDraftInput({faces:[{name:'Teacher',oracleText:original}]},[def]).faces[0];assert.ok(face.oracleText.includes(`"${cost}, Exile this card from your graveyard:`));assert.equal(face.customKeywordIdentities,undefined);assert.equal(face.displayOracleText,original);
 }
});
test('derived casting-program prices bind the granted source without an activation or duration',()=>{
 const def=validateMechanic({keyword:'Reecho {Cost}',reminder:"As you cast an instant or sorcery spell, you may exile this card from your graveyard and pay {Cost}. If you do, add this card's effects to that spell. You may exile only one card this way per spell."});
 const original='Each instant and sorcery card in your graveyard has reecho. The reecho cost is equal to its mana cost.';
 const face=compileDraftInput({faces:[{name:'Teacher',oracleText:original}]},[def]).faces[0];assert.ok(face.oracleText.includes("and pay this card's mana cost."));assert.ok(!face.oracleText.includes('undefined'));assert.equal(face.customKeywordIdentities,undefined);assert.equal(face.displayOracleText,original);
});
