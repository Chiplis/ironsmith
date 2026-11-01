import test from 'node:test';
import assert from 'node:assert/strict';
import { validateMechanic, compileDraftInput, draftMechanics } from '../src/lib/custom-mechanics.js';
import { REMINDER_TEMPLATES } from '../src/lib/mechanics-catalog.js';
const definition = keyword => validateMechanic({ keyword: `${keyword} {Cost}`, reminder: `Whenever you cast a spell, you may pay {Cost} to ${keyword} this creature until end of turn.` });
for (const [keyword, status] of [['bribe','bribed'], ['enchantify','enchantified'], ['moon bless','moon blessed']]) {
  test(`named status ${keyword} expands applications, grants and references without changing source text`, () => {
    const def = validateMechanic({...definition(keyword), forms: { designation: status }});
    const text = `${keyword} {1}{B}\nAs long as this creature is ${status}, other Advisors you control have menace.\nOther Advisors you control have ${keyword} {2} and "Whenever this creature attacks while ${status}, draw a card."`;
    const face = compileDraftInput({faces:[{oracleText:text}]}, [def]).faces[0];
    assert.equal(face.displayOracleText, text);
    assert.match(face.oracleText, new RegExp(`pay \\{1\\}\\{B\\}\\. If you do, give this creature the ${status} status until end of turn`));
    assert.ok(face.oracleText.includes(`this creature has the ${status} status`));
    assert.ok(face.oracleText.includes(`have "Whenever you cast a spell, you may pay {2}. If you do, give this creature the ${status} status until end of turn."`));
    assert.ok(face.oracleText.includes(`this creature with the ${status} status attacks`));
    const watcher = {faces:[{oracleText:`Whenever a creature you control becomes ${status}, you may return this card from your graveyard to your hand.`}]};
    assert.equal(draftMechanics(watcher,[def]).length,1);
    assert.ok(compileDraftInput(watcher,[def]).faces[0].oracleText.includes(`gains the ${status} status`));
    assert.equal(compileDraftInput(watcher,[def]).faces[0].customKeywordIdentities, undefined);
  });
}
test('temporary status template expands a payable triggered ability', () => {
  const template = REMINDER_TEMPLATES.find(t => t.kind === 'Temporary status');
  assert.ok(template);
  const def = validateMechanic(template);
  assert.match(compileDraftInput({faces:[{oracleText:template.usage}]},[def]).faces[0].oracleText, /give this creature the bribed status until end of turn/);
});
test('explicit custom status forms and negative predicates remain distinct', () => {
  const def = validateMechanic({...definition('bless'), forms:{designation:'hallowed',participle:'hallowed'}});
  const result = compileDraftInput({faces:[{oracleText:'Bless {0}\nIf this creature is not hallowed, draw a card.\nHallowed creatures you control get +1/+1.'}]},[def]);
  assert.match(result.faces[0].oracleText,/does not have the hallowed status/);
  assert.match(result.faces[0].oracleText,/creatures with the hallowed status you control/);
});
test('parameterless status abilities can be granted to other permanents', () => {
  const def = validateMechanic({keyword:'bless', reminder:'When this creature enters, bless this creature until end of turn.'});
  const text = compileDraftInput({faces:[{oracleText:'Other creatures you control have bless.'}]},[def]).faces[0].oracleText;
  assert.equal(text, 'Other creatures you control have "When this creature enters, give this creature the blessed status until end of turn.".');
});
