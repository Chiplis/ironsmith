import test from 'node:test';
import assert from 'node:assert/strict';
import { KEYWORD_FORM_DERIVATION_FIXTURES } from '../src/lib/keyword-forms.generated.js';
import { deriveParticiple, officialKeywordForms, officialFormsWithCostNoun, customKeywordForms, surfaceForms } from '../src/lib/keyword-forms.js';
import { validateMechanic, compileDraftInput, mergeMechanics } from '../src/lib/custom-mechanics.js';
import { bindReminderCost, bindReminderCostFromDeclaration, hasReminderCostReference } from '../src/lib/reminder-costs.js';
import { getMtgKeywordRule } from '../src/lib/mtg-keywords.js';

test('the web derivation rule matches the compiler lexicon fixtures', () => {
  for (const [name, participle] of KEYWORD_FORM_DERIVATION_FIXTURES) assert.equal(deriveParticiple(name), participle, name);
});

test('official forms come from one table, including irregular and phrasal forms', () => {
  assert.equal(officialKeywordForms('Foretell').participle, 'foretold');
  assert.equal(officialKeywordForms('Kicker').designation, 'kicked');
  assert.equal(officialKeywordForms('Beam me up').participle, 'beamed up');
  assert.deepEqual(officialFormsWithCostNoun('upkeep').map(forms => forms.name), ['cumulative upkeep']);
  assert.ok(surfaceForms(officialKeywordForms('Phasing')).includes('phased out'));
  // The UI catalog gains the lexicon's forms, so "foretold" names Foretell.
  assert.equal(getMtgKeywordRule('foretold')?.title, 'Foretell');
});

test('custom forms are derived from the name unless the definition overrides them', () => {
  assert.deepEqual(customKeywordForms({ keyword: 'Blink' }), { name: 'blink', participle: 'blinked', designation: null, costNoun: 'blink', variants: [] });
  const overridden = customKeywordForms({ keyword: 'flicker step', forms: { participle: 'Flickered', costNoun: 'flicker' } });
  assert.equal(overridden.participle, 'flickered');
  assert.equal(overridden.costNoun, 'flicker');
  assert.equal(validateMechanic({ keyword: 'flicker step', reminder: 'As an additional cost to cast this spell, you may pay its flicker cost.', forms: { participle: 'flickered', costNoun: 'flicker' } }).forms.costNoun, 'flicker');
  assert.equal(validateMechanic({ keyword: 'blink', reminder: 'Draw a card.' }).forms, undefined);
  assert.throws(() => validateMechanic({ keyword: 'blink', reminder: 'Draw a card.', forms: { plural: 'blinks' } }), /Unknown custom keyword form/);
});

test('custom keywords never take official names or forms', () => {
  for (const keyword of ['foretold', 'kicked', 'beamed up', 'monstrous']) {
    assert.throws(() => validateMechanic({ keyword, reminder: 'Draw a card.' }), /official keywords cannot be redefined/, keyword);
  }
  // "Kick" derives "kicked", Kicker's participle: a reference would be ambiguous.
  assert.throws(() => validateMechanic({ keyword: 'kick', reminder: 'Draw a card.' }), /belongs to an official keyword/);
  assert.equal(validateMechanic({ keyword: 'kick', reminder: 'Draw a card.', forms: { participle: 'kicked off' } }).keyword, 'kick');
  assert.throws(() => validateMechanic({ keyword: 'stall', reminder: 'Draw a card.', forms: { costNoun: 'upkeep' } }), /belongs to an official keyword/);
});

test('a custom cost noun binds through the same reference reader as official ones', () => {
  const forms = customKeywordForms({ keyword: 'flicker step', forms: { costNoun: 'flicker' } });
  const reminder = 'As an additional cost to cast this spell, you may pay its flicker cost.';
  assert.ok(hasReminderCostReference(reminder, 'flicker step', forms));
  assert.ok(!hasReminderCostReference(reminder, 'flicker step'));
  assert.equal(bindReminderCost(reminder, 'flicker step', '{2}', forms), 'As an additional cost to cast this spell, you may pay {2}.');
  // Official cost nouns resolve their keyword through the lexicon.
  const upkeep = bindReminderCostFromDeclaration('At the beginning of your upkeep, put an age counter on this permanent, then sacrifice it unless you pay its upkeep cost for each age counter on it.', 'Cumulative upkeep {1}');
  assert.equal(upkeep.binding.cost, '{1}');
});

const blink = { keyword: 'blink', reminder: 'As an additional cost to cast this spell, you may pay its blink cost.' };

test('a custom participle binds to the payment its definition creates, never to kicker', () => {
  const [definition] = mergeMechanics([], [blink]);
  const [face] = compileDraftInput({ faces: [{ name: 'Blinker', oracleText: 'Blink {1}{U}\nWhen this creature enters, if it was blinked, draw a card.\nWhenever you cast a spell, if it was kicked, you gain 1 life.' }] }, [definition]).faces;
  assert.equal(face.oracleText, "As an additional cost to cast this spell, you may pay {1}{U}.\nWhen this creature enters, if its additional cost was paid, draw a card.\nWhenever you cast a spell, if it was kicked, you gain 1 life.");
  assert.deepEqual(face.customKeywordIdentities, ['blink']);
  assert.equal(face.displayOracleText, 'Blink {1}{U}\nWhen this creature enters, if it was blinked, draw a card.\nWhenever you cast a spell, if it was kicked, you gain 1 life.');

  const [spell] = compileDraftInput({ faces: [{ name: 'Bolt', oracleText: 'Blink {2}\nThis spell deals 2 damage to any target. If this spell wasn\'t blinked, you gain 1 life.' }] }, [definition]).faces;
  assert.match(spell.oracleText, /If this spell's additional cost wasn't paid, you gain 1 life\./);
});

test('participle overrides bind, and unbindable participles are left for strict compilation', () => {
  const [step] = mergeMechanics([], [{ keyword: 'flicker step', reminder: 'As an additional cost to cast this spell, you may pay its flicker cost.', forms: { participle: 'flickered', costNoun: 'flicker' } }]);
  const [face] = compileDraftInput({ faces: [{ name: 'Step', oracleText: 'Flicker step {2}\nDraw a card. If this spell was flickered, draw another card.' }] }, [step]).faces;
  assert.equal(face.oracleText, "As an additional cost to cast this spell, you may pay {2}.\nDraw a card. If this spell's additional cost was paid, draw another card.");

  // A second optional payment makes "its additional cost" ambiguous: decline.
  const [definition] = mergeMechanics([], [blink]);
  const [ambiguous] = compileDraftInput({ faces: [{ name: 'Two', oracleText: 'Blink {1}\nYou may pay an additional {2} as you cast this spell.\nWhen this creature enters, if it was blinked, draw a card.' }] }, [definition]).faces;
  assert.match(ambiguous.oracleText, /if it was blinked/);
  // An action definition gives the participle no state to test: decline.
  const [shift] = mergeMechanics([], [{ keyword: 'shift', reminder: 'Draw a card, then discard a card.' }]);
  const [action] = compileDraftInput({ faces: [{ name: 'Act', oracleText: 'Shift.\nIf this spell was shifted, draw a card.' }] }, [shift]).faces;
  assert.match(action.oracleText, /If this spell was shifted/);
});
