import test from 'node:test';
import assert from 'node:assert/strict';
import { compileDraftInput, installCustomMechanics, mergeMechanics, prepareCustomCard, splitMechanicName, validateMechanic } from '../src/lib/custom-mechanics.js';
import { getMtgKeywordHelp, splitTextWithMtgKeywordRules } from '../src/lib/mtg-keywords.js';
import { parameterExample, parseParameterDeclaration, reminderParameters, hasReminderParameters } from '../src/lib/reminder-costs.js';
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
  // The timed exile is this keyword's payment, so its action is named "Foresee".
  assert.deepEqual(face.customKeywordPayments, ['foresee']);
  assert.equal(face.displayOracleText, 'Foresee 3—{1}{U}\nFlying');
  assert.throws(() => compileDraftInput({ faces: [{ name: 'Bare', oracleText: 'Foresee' }] }, [definition]), /for example "Foresee 3—\{1\}\{U\}"/);
});

test('a card\'s keyword helper explains the mechanic with the values it declares', () => {
  installCustomMechanics([foresee]);
  const [keyword] = splitTextWithMtgKeywordRules('Foresee 3—{1}{U}\nFlying').filter(segment => segment.type === 'keyword' && segment.rule.custom);
  const { description } = getMtgKeywordHelp(keyword.text, keyword.rule, 'en', keyword.context);
  assert.match(description, /^Rather than cast this card from your hand, you may pay \{1\}\{U\} and exile it with 3 progress counters on it\./);
});

test('a dash between parameters is decoration: a space separates them too', () => {
  for (const [declaration, values] of [
    ['Foresee 3 {1}{U}', { number: '3', cost: '{1}{U}' }],
    ['Foresee {1}{U} 3', { cost: '{1}{U}', number: '3' }],
    ['Foresee 3 - {1}{U}', { number: '3', cost: '{1}{U}' }],
    ['Foresee X pay 3 life', { number: 'X', cost: 'pay 3 life' }],
  ]) assert.deepEqual(parseParameterDeclaration(foresee.reminder, 'foresee', declaration), values, declaration);
  const [definition] = mergeMechanics([], [foresee]);
  const [face] = compileDraftInput({ faces: [{ name: 'Construct', oracleText: 'Foresee 3 {1}{U}' }] }, [definition]).faces;
  assert.match(face.oracleText, /you may pay \{1\}\{U\} and exile it with 3 progress counters on it\./);
  // The card keeps the layout it was printed with.
  assert.equal(face.displayOracleText, 'Foresee 3 {1}{U}');
});

const herald = {
  keyword: 'Herald a {Type}',
  reminder: 'As an additional cost to cast this spell, you may reveal a {Type} card from your hand.',
};

test('the name carries the card line layout after the keyword', () => {
  assert.deepEqual(splitMechanicName('Muster {Number} {Cost}'), { keyword: 'muster', template: '{Number} {Cost}' });
  assert.deepEqual(splitMechanicName('Muster — {Number}'), { keyword: 'muster', template: '— {Number}' });
  assert.deepEqual(splitMechanicName('Muster'), { keyword: 'muster', template: null });
  const def = validateMechanic({ keyword: 'Muster {Number}—{Cost}', reminder: foresee.reminder });
  assert.equal(def.keyword, 'muster');
  assert.equal(def.template, '{Number}—{Cost}');
  // Saved definitions keep the layout as their own field.
  assert.equal(validateMechanic(def).template, '{Number}—{Cost}');
  // The layout fixes the order; its dash still reads as a space.
  assert.deepEqual(parseParameterDeclaration(foresee.reminder, 'muster', 'Muster 2 {1}{U}', def.template), { number: '2', cost: '{1}{U}' });
  assert.equal(parseParameterDeclaration(foresee.reminder, 'muster', 'Muster {1}{U} 2', def.template), null);
  assert.equal(parameterExample(foresee.reminder, 'Muster', def.template), 'Muster 3—{1}{U}');
  assert.equal(parameterExample(foresee.reminder, 'Muster', '{Cost} {Number}'), 'Muster {1}{U} 3');
});

test('a layout must place exactly the reminder\'s parameters', () => {
  assert.throws(() => validateMechanic({ keyword: 'Muster {Number}', reminder: foresee.reminder }), /must place it/);
  assert.throws(() => validateMechanic({ keyword: 'Muster {Number} {Type}', reminder: 'Put {Number} +1/+1 counters on this creature.' }), /never uses it/);
  assert.throws(() => validateMechanic({ keyword: 'Muster {Number} {Number}', reminder: 'Put {Number} +1/+1 counters on this creature.' }), /more than once/);
  assert.throws(() => validateMechanic({ keyword: 'Muster {Number}{Cost}', reminder: foresee.reminder }), /space or a dash/);
  assert.throws(() => validateMechanic({ keyword: 'Muster {Amount}', reminder: 'Draw a card.' }), /not a parameter/);
});

test('{Type} takes a card type from the declaration and fixes its article', () => {
  assert.deepEqual(reminderParameters(herald.reminder), ['type']);
  assert.ok(hasReminderParameters(herald.reminder));
  const [definition] = mergeMechanics([], [herald]);
  assert.equal(definition.keyword, 'herald');
  const compiled = compileDraftInput({ faces: [
    { name: 'A', oracleText: 'Herald a Goblin\nFlying' },
    { name: 'B', oracleText: 'Herald an Elf Warrior' },
    { name: 'C', oracleText: 'Herald an artifact' },
  ] }, [definition]).faces;
  assert.equal(compiled[0].oracleText, 'As an additional cost to cast this spell, you may reveal a Goblin card from your hand.\nFlying');
  assert.equal(compiled[1].oracleText, 'As an additional cost to cast this spell, you may reveal an Elf Warrior card from your hand.');
  assert.equal(compiled[2].oracleText, 'As an additional cost to cast this spell, you may reveal an artifact card from your hand.');
  assert.deepEqual(compiled[0].customKeywordIdentities, ['herald']);
  assert.throws(() => compileDraftInput({ faces: [{ name: 'Bare', oracleText: 'Herald' }] }, [definition]), /for example "Herald a Goblin"/);
});

test('a card defining a typed mechanic in its reminder infers the layout', async () => {
  const draft = { faces: [{ name: 'Scout', oracleText: 'Rally an Elf (As an additional cost to cast this spell, you may reveal a {Type} card from your hand.)' }] };
  const prepared = await prepareCustomCard(draft, async input => {
    if (!input.customMechanics.length) throw new Error('unsupported line family');
    assert.equal(input.faces[0].oracleText, 'As an additional cost to cast this spell, you may reveal an Elf card from your hand.');
    return { canCreate: true };
  });
  assert.equal(prepared.draft.customMechanics[0].keyword, 'rally');
});
