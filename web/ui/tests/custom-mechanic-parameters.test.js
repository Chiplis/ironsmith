import test from 'node:test';
import assert from 'node:assert/strict';
import { compileDraftInput, mergeMechanics, validateMechanic } from '../src/lib/custom-mechanics.js';
import { parseParameterDeclaration, reminderParameters, hasReminderParameters } from '../src/lib/reminder-costs.js';
import { expandKnownReminderDefinition } from '../src/lib/official-reminder-definitions.js';

const foresee = {
  keyword: 'foresee',
  reminder: 'Rather than cast this card from your hand, you may pay {Cost} and exile it with {Number} progress counters on it. At the beginning of your upkeep, or whenever an artifact or creature you control enters, remove a progress counter. When the last is removed, you may cast it without paying its mana cost.',
};

test('a reminder names its parameters with {Cost} and {Number}', () => {
  assert.deepEqual(reminderParameters(foresee.reminder), ['cost', 'number']);
  assert.ok(hasReminderParameters(foresee.reminder));
  // {Cost} alone keeps the existing price binder.
  assert.ok(!hasReminderParameters('As an additional cost to cast this spell, you may pay {Cost}.'));
  assert.throws(() => validateMechanic({ keyword: 'foresee', reminder: 'Exile it with {Amount} counters.' }), /\{Number\} for an amount/);
});

test('a declaration gives the amount and price in either order', () => {
  assert.deepEqual(parseParameterDeclaration(foresee.reminder, 'foresee', 'Foresee 3—{1}{U}'), { number: '3', cost: '{1}{U}' });
  assert.deepEqual(parseParameterDeclaration(foresee.reminder, 'foresee', 'Foresee {1}{U}—3'), { cost: '{1}{U}', number: '3' });
  assert.equal(parseParameterDeclaration(foresee.reminder, 'foresee', 'Foresee 3'), null);
  assert.equal(parseParameterDeclaration(foresee.reminder, 'foresee', 'Foresee three—{1}{U}'), null);
  assert.deepEqual(parseParameterDeclaration('Put {Number} +1/+1 counters on this creature.', 'grow', 'Grow 2'), { number: '2' });
});

test('a timed exile with its own counter and extra removal events reads as the full rule', () => {
  const expanded = expandKnownReminderDefinition(foresee.reminder.replace('{Cost}', '{1}{U}').replace('{Number}', '3'));
  assert.equal(expanded, [
    "If you could begin to cast this card from your hand, you may pay {1}{U} and exile it with 3 progress counters on it. This action doesn't use the stack. At the beginning of your upkeep, if this card is exiled with a progress counter on it, remove a progress counter from it. When the last progress counter is removed from this card, if it's exiled, you may play it without paying its mana cost.",
    'Whenever an artifact or creature you control enters, if this card is exiled with a progress counter on it, remove a progress counter from it.',
  ].join('\n'));
  // An extra event that is not a trigger is left for strict compilation.
  const odd = foresee.reminder.replace('or whenever an artifact or creature you control enters', 'or during combat').replace('{Cost}', '{1}').replace('{Number}', '2');
  assert.equal(expandKnownReminderDefinition(odd), odd);
});

test('a card declaring the mechanic binds its parameters and keeps only custom identity', () => {
  const [definition] = mergeMechanics([], [foresee]);
  const [face] = compileDraftInput({ faces: [{ name: 'Construct', oracleText: 'Foresee 3—{1}{U}\nFlying' }] }, [definition]).faces;
  assert.match(face.oracleText, /^If you could begin to cast this card from your hand, you may pay \{1\}\{U\} and exile it with 3 progress counters on it\./);
  assert.match(face.oracleText, /\nWhenever an artifact or creature you control enters, if this card is exiled with a progress counter on it, remove a progress counter from it\.\nFlying$/);
  assert.deepEqual(face.customKeywordIdentities, ['foresee']);
  assert.equal(face.displayOracleText, 'Foresee 3—{1}{U}\nFlying');
  assert.throws(() => compileDraftInput({ faces: [{ name: 'Bare', oracleText: 'Foresee' }] }, [definition]), /for example "Foresee 3—\{1\}\{U\}"/);
});
