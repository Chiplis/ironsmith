import test from 'node:test';
import assert from 'node:assert/strict';
import { validateMechanic, mergeMechanics, draftMechanics } from '../src/lib/custom-mechanics.js';
import { MTG_KEYWORD_RULES } from '../src/lib/mtg-keyword-rules.generated.js';
const reminder = 'Create a 0/0 white Soldier Army creature token. Then put a +1/+1 counter on each Army you control.';
test('official registry reserves Recruit independently of engine support', () => {
  assert.ok(MTG_KEYWORD_RULES.some(rule => rule.title === 'Recruit'));
  for (const keyword of ['Recruit', ' RECRUIT ', 'Ｒｅｃｒｕｉｔ', 'Flying', 'kicked', 'recruited']) {
    for (const extra of [{}, {sourceScope:'msem'}, {officialCostDeclaration:true, template:'{Cost}', reminder:'{Cost}: Draw a card.'}]) {
      assert.throws(() => validateMechanic({keyword, reminder, ...extra}), /official keywords cannot be redefined/);
    }
  }
});
test('card and set imports reject colliding definitions', () => {
  const def = {keyword:'Recruit', reminder, sourceScope:'msem'};
  assert.throws(() => mergeMechanics([], [def]), /official keywords/);
  assert.throws(() => draftMechanics({customMechanics:[def], faces:[{name:'Custom', oracleText:'Recruit.'}]}), /official keywords/);
});
test('aliases are reserved without substring matching', () => {
  for (const field of ['participle', 'designation', 'costNoun', 'variants']) {
    assert.throws(() => validateMechanic({keyword:'Muster ranks', reminder, forms:{[field]:field === 'variants' ? ['Recruit'] : 'Recruit'}}), /belongs to an official keyword/);
  }
  assert.equal(validateMechanic({keyword:'Muster ranks', reminder}).keyword, 'muster ranks');
  assert.equal(validateMechanic({keyword:'Recruitment', reminder}).keyword, 'recruitment');
});
