import test from 'node:test';
import assert from 'node:assert/strict';
import { validateMechanic, prepareCustomCard, mergeMechanics, draftMechanics, compileDraftInput, installCustomMechanics } from '../src/lib/custom-mechanics.js';
import { getMtgKeywordRule, getMtgKeywordHelp, splitTextWithMtgKeywordRules } from '../src/lib/mtg-keywords.js';
import { variukDraft } from './custom-mechanics-fixture.mjs';
import { normalizeDraftForApi } from '../src/lib/custom-card-draft.js';

test('custom names survive API normalization separately from expanded behavior', () => {
  const definitions = mergeMechanics([], [
    { keyword: 'soar', reminder: "This creature can't be blocked except by creatures with flying or reach." },
    { keyword: 'watchful', reminder: "Attacking doesn't cause this creature to tap." },
  ]);
  const compiled = compileDraftInput({ faces: [
    { name: 'A', oracleText: 'Soar.' },
    { name: 'B', oracleText: 'Watchful.' },
    { name: 'C', oracleText: 'Flying' },
  ] }, definitions);
  const input = normalizeDraftForApi(compiled);
  assert.deepEqual(input.faces[0].customKeywordIdentities, ['soar']);
  assert.deepEqual(input.faces[1].customKeywordIdentities, ['watchful']);
  assert.equal(input.faces[2].customKeywordIdentities, undefined);
  assert.match(input.faces[0].oracleText, /can't be blocked/);
  assert.equal(input.faces[0].displayOracleText, 'Soar.');
  const edited = compileDraftInput({ faces: [{
    name: 'A', oracleText: 'Flying', customKeywordIdentities: ['soar'],
  }] }, definitions);
  assert.equal(normalizeDraftForApi(edited).faces[0].customKeywordIdentities, undefined);
});

const definition = { keyword: 'shift', reminder: 'Noncreature, nonartifact spells you cast cost 1 less to cast until end of turn.', sourceCard: 'Failed Variuk' };

test('the two screenshot definitions conflict because only Variuk restricts spells to you', async () => {
  const known = [{ ...definition, reminder: 'Noncreature, nonartifact spells cost {1} less to cast until end of turn.' }];
  await assert.rejects(prepareCustomCard(variukDraft(), async () => {
    assert.fail('conflicting definitions must fail before compiling');
  }, { known }), /already defined it/);
});

test('fallback discovery is independent of the keyword name and compiler diagnostic', async () => {
  for (const keyword of ['glimmer', 'emberwake', 'moonpulse']) {
    for (const diagnostic of ['unsupported line family', 'unsupported target phrase', 'unrelated earlier parse failure']) {
      const draft = { faces: [{ name: 'Example', oracleText: `Example deals 2 damage to any target, ${keyword}. (Draw a card.)` }] };
      let calls = 0;
      const prepared = await prepareCustomCard(draft, async input => {
        calls++;
        if (!input.customMechanics.length) throw new Error(diagnostic);
        assert.equal(input.faces[0].oracleText, 'Example deals 2 damage to any target, Draw a card.');
        return { canCreate: true };
      });
      assert.equal(calls, 2);
      assert.equal(prepared.draft.customMechanics[0].keyword, keyword);
    }
  }
});

test('discovers shift from the diagnostic, reserves it, and retries the full card', async () => {
  let calls = 0;
  const reserved = [];
  const prepared = await prepareCustomCard(variukDraft(), async input => {
    calls++;
    if (input.faces[0].oracleText.includes(' shift,')) throw new Error("could not find verb in effect clause (clause: 'shift'; known verbs: draw)");
    assert.match(input.faces[0].oracleText, /noncreature, nonartifact spells you cast this turn cost \{1\} less to cast/i);
    assert.match(input.faces[0].displayOracleText, /shift, then put/);
    assert.ok(!input.faces[0].displayOracleText.includes('Noncreature'));
    return { canCreate: true };
  }, { reserve: async defs => reserved.push(...defs) });
  assert.equal(calls, 2);
  assert.equal(prepared.draft.customMechanics[0].keyword, 'shift');
  assert.ok(reserved.some(def => def.keyword === 'shift'));
  assert.match(prepared.draft.faces[0].oracleText, /\(Noncreature/);
});

test('reserves the definition even when a later clause fails compilation', async () => {
  let retained = [];
  await assert.rejects(prepareCustomCard(variukDraft(), async input => {
    if (!input.customMechanics.length) throw new Error("could not find verb in effect clause (clause: 'shift'; known verbs: draw)");
    throw new Error('unsupported subsequent effect');
  }, { reserve: async defs => { retained = defs; } }), /unsupported subsequent effect/);
  assert.equal(retained[0].keyword, 'shift');
  assert.throws(() => mergeMechanics(retained, [{ ...definition, reminder: 'Draw a card.' }]), /retained even when a card fails/);
});

test('a known name with a different reminder conflicts before compilation', () => {
  const draft = variukDraft();
  draft.faces[0].oracleText = draft.faces[0].oracleText.replace('cost 1 less', 'cost 2 less');
  assert.throws(() => draftMechanics(draft, [definition]), /rename this mechanic/);
  assert.equal(mergeMechanics([definition], [{ ...definition, keyword: ' SHIFT ', reminder: definition.reminder.toUpperCase() }]).length, 1);
});

test('does not redefine official keywords or infer an ambiguous reminder', async () => {
  assert.throws(() => mergeMechanics([], [{ ...definition, keyword: 'flying' }]), /official keywords/);
  const draft = variukDraft();
  draft.faces[0].oracleText += ' (Draw a card.)';
  await assert.rejects(prepareCustomCard(draft, async () => { throw new Error("could not find verb in effect clause (clause: 'shift'; known verbs: draw)"); }), /more than one possible reminder/);
});

test('substitution respects complete action clauses and helper indexes refresh', () => {
  const draft = variukDraft();
  draft.faces[0].oracleText += '\nWhenever a creature named Shift enters, draw a card.';
  const input = compileDraftInput(draft, mergeMechanics([], [definition]));
  assert.match(input.faces[0].oracleText, /named Shift enters/);
  assert.equal(getMtgKeywordRule('shift'), null);
  installCustomMechanics([definition]);
  const rule = getMtgKeywordRule('shift');
  assert.equal(rule.custom, true);
  assert.equal(getMtgKeywordHelp('shift', rule).description, definition.reminder);
  assert.ok(splitTextWithMtgKeywordRules('shift, then put a counter').some(segment => segment.type === 'keyword' && segment.text === 'shift'));
});

test('an earlier unrelated parse error still reserves an explicit custom declaration', async () => {
  let retained = [];
  const draft = variukDraft();
  draft.faces[0].oracleText = 'This line cannot compile.\n' + draft.faces[0].oracleText;
  await assert.rejects(prepareCustomCard(draft, async () => { throw new Error('unsupported line'); }, {
    reserve: async definitions => { retained = definitions; },
  }), /unsupported line/);
  assert.equal(retained[0].keyword, 'shift');
});

test('macro expansion does not rewrite unrelated then clauses or quoted rules', () => {
  const draft = variukDraft();
  draft.faces[0].oracleText += '\nDraw a card, then discard a card.\nCreate a token with "{T}: shift, then draw a card."';
  const prepared = compileDraftInput(draft, mergeMechanics([], [definition]));
  assert.match(prepared.faces[0].oracleText, /Draw a card, then discard a card/);
  assert.match(prepared.faces[0].oracleText, /"\{T\}: shift, then draw a card\."/);
});

test('named cost slots retain every payment and bind separately at each use', () => {
  const definitions = mergeMechanics([], [{ keyword: 'recur', reminder: 'You may cast this card from your graveyard for its recur cost.' }]);
  const input = compileDraftInput({ faces: [{ name: 'A', oracleText: 'Recur—{2}{B}, Exile three other cards from your graveyard.\nRecur {5}{B}' }] }, definitions);
  assert.equal(input.faces[0].oracleText, 'You may cast this card from your graveyard for {2}{B}, Exile three other cards from your graveyard.\nYou may cast this card from your graveyard for {5}{B}.');
  assert.deepEqual(input.faces[0].customKeywordIdentities, ['recur']);
  assert.match(input.faces[0].displayOracleText, /^Recur—/);
  assert.throws(() => compileDraftInput({ faces: [{ name: 'A', oracleText: 'Recur' }] }, definitions), /requires a declared cost/);
});

test('costed custom definitions are discovered and reused without fixing one price', async () => {
  const reminder = 'You may cast this card from your graveyard for its recur cost.';
  const draft = { faces: [{ name: 'A', oracleText: `Recur—{2}{B}, Exile three other cards from your graveyard. (${reminder})` }] };
  let calls = 0;
  const prepared = await prepareCustomCard(draft, async input => {
    calls++;
    if (!input.customMechanics.length) throw new Error('unsupported line family');
    assert.equal(input.faces[0].oracleText, 'You may cast this card from your graveyard for {2}{B}, Exile three other cards from your graveyard.');
    return { canCreate: true };
  });
  assert.equal(calls, 2);
  const second = compileDraftInput({ faces: [{ name: 'B', oracleText: 'Recur {1}{U}' }] }, prepared.draft.customMechanics);
  assert.equal(second.faces[0].oracleText, 'You may cast this card from your graveyard for {1}{U}.');
});

test('custom variable definitions bind per use and preserve custom identity', async () => {
  const reminder = 'Whenever this creature attacks, add X {R}. This mana lasts until end of combat.';
  const draft = { faces: [{ name: 'A', oracleText: `Battle fuel X, where X is this creature's power. (${reminder})` }] };
  const prepared = await prepareCustomCard(draft, async input => {
    if (!input.customMechanics.length) throw new Error('unsupported line family');
    assert.match(input.faces[0].oracleText, /add X \{R\}, where X is this creature's power/);
    assert.deepEqual(input.faces[0].customKeywordIdentities, ['battle fuel']);
    return { canCreate: true };
  });
  const second = compileDraftInput({ faces: [{ name: 'B', oracleText: "Battle fuel X, where X is this creature's toughness." }] }, prepared.draft.customMechanics);
  assert.match(second.faces[0].oracleText, /where X is this creature's toughness/);
  assert.match(second.faces[0].displayOracleText, /^Battle fuel X/);
});

test('custom paid-copy definitions restore the complete payment and bind each use', async () => {
  const reminder = 'When you cast this spell, copy it for each time you paid {2}. You may choose new targets for the copies. Copies become tokens.';
  const draft = { faces: [{ name: 'A', oracleText: `Resound {2} (${reminder})` }] };
  const prepared = await prepareCustomCard(draft, async input => {
    if (!input.customMechanics.length) throw new Error('unsupported line family');
    assert.match(input.faces[0].oracleText, /you may pay \{2\} any number of times/);
    assert.deepEqual(input.faces[0].customKeywordIdentities, ['resound']);
    return { canCreate: true };
  });
  const second = compileDraftInput({ faces: [{ name: 'B', oracleText: 'Resound—Discard a card' }] }, prepared.draft.customMechanics);
  assert.match(second.faces[0].oracleText, /you may discard a card any number of times/);
  assert.match(second.faces[0].oracleText, /for each time this spell's additional cost was paid/);
  assert.deepEqual(second.faces[0].customKeywordIdentities, ['resound']);
});

test('custom activation definitions retain their costs, effects, and separate identities', () => {
  for (const [keyword, reminder, restriction] of [
    ['final flourish', 'Activate each final flourish ability only once.', 'Activate this ability only once.'],
    ['battle promise', 'Activate only if this creature attacked this turn and only once each turn.', 'Activate only if this creature attacked this turn and only once each turn.'],
  ]) {
    const definitions = mergeMechanics([], [{ keyword, reminder }]);
    const input = compileDraftInput({ faces: [{ name: 'Performer', oracleText: `${keyword} — {1}, Pay 2 life: Draw a card. (${reminder})` }] }, definitions);
    assert.equal(input.faces[0].oracleText, `{1}, Pay 2 life: Draw a card. ${restriction}`);
    assert.deepEqual(input.faces[0].customKeywordIdentities, [keyword]);
    assert.equal(input.faces[0].displayOracleText, `${keyword} — {1}, Pay 2 life: Draw a card.`);
  }
});

test('custom face-down action substitutes only its bound operand and retains subsequent actions', () => {
  const reminder = "To obscure a card, put it onto the battlefield face down as a 2/2 creature with ward {2}. Turn it face up any time for its mana cost if it's a creature card.";
  const known = mergeMechanics([], [{ keyword: 'obscure', reminder }]);
  const result = compileDraftInput({ faces: [{ name: 'Hidden', oracleText: 'Obscure the top card of your library, then draw a card.' }] }, known).faces[0];
  assert.match(result.oracleText, /Put the top card of your library onto the battlefield face down/);
  assert.match(result.oracleText, /Turn it face up any time/);
  assert.match(result.oracleText, /Then draw a card/);
  assert.deepEqual(result.customKeywordIdentities, ['obscure']);
  assert.equal(result.displayOracleText, 'Obscure the top card of your library, then draw a card.');
});

test('a custom self cost shares the untap commitment without native identity', () => {
  const known = mergeMechanics([], [{ keyword: 'strain', reminder: "This creature won't untap during your next untap step." }]);
  const face = compileDraftInput({ faces: [{ name: 'Worker', oracleText: '{1}, {T}, Strain this creature: Draw a card.' }] }, known).faces[0];
  assert.equal(face.oracleText, '{1}, {T}, Choose to have this creature not untap during your next untap step: Draw a card.');
  assert.deepEqual(face.customKeywordIdentities, ['strain']);
});

test('formal exile actions retain their own identity and following instructions', () => {
  const reminder = "To displace a creature, exile it. While it's exiled, its owner may cast it for {2} rather than its mana cost.";
  const known = mergeMechanics([], [{ keyword: 'displace', reminder }]);
  const face = compileDraftInput({ faces: [{ name: 'Portal', oracleText: 'Displace up to two target creatures, then draw a card.' }] }, known).faces[0];
  assert.equal(face.oracleText, "Exile up to two target creatures. While it's exiled, its owner may cast it for {2} rather than its mana cost. Then draw a card.");
  assert.deepEqual(face.customKeywordIdentities, ['displace']);
});

test('custom material definitions carry their declared filter into compilation', () => {
  const reminder = "{8}{U}, Exile this artifact, Exile the four or more from among other permanents you control and/or cards in your graveyard: Return this card transformed under its owner's control. Reforge only as a sorcery.";
  const known = mergeMechanics([], [{ keyword: 'reforge', reminder }]);
  const face = compileDraftInput({ faces: [{ name: 'Forge', oracleText: 'Reforge with four or more nonlands with activated abilities {8}{U}' }] }, known).faces[0];
  assert.match(face.oracleText, /Exile four or more nonlands with activated abilities from among/);
  assert.deepEqual(face.customKeywordIdentities, ['reforge']);
});

test('custom names receive full unambiguous official definitions without official identity', async () => {
  const { MTG_KEYWORD_REMINDERS } = await import('../src/lib/mtg-keyword-reminders.generated.js');
  for (const [source, keyword, expected] of [['nightbound', 'moonward', /back face up, transform it/], ['assist', 'collaborate', /before activating mana abilities/]]) {
    const reminder = MTG_KEYWORD_REMINDERS[source].text;
    const def = validateMechanic({ keyword, reminder });
    assert.match(def.expansion, expected);
    assert.equal(def.keyword, keyword);
    assert.equal(def.reminder, reminder);
  }
});

test('{Cost} binds to the declared price, lowercasing an action price mid-sentence', () => {
  const definitions = mergeMechanics([], [
    { keyword: 'blink', reminder: "{Cost}: Exile this creature, then return it to the battlefield under its owner's control." },
    { keyword: 'rush', reminder: "You may pay {Cost} rather than pay this spell's mana cost." },
  ]);
  const faces = compileDraftInput({ faces: [
    { name: 'A', oracleText: 'Blink—Sacrifice another creature.' },
    { name: 'B', oracleText: 'Rush—Sacrifice a creature.' },
  ] }, definitions).faces;
  assert.match(faces[0].oracleText, /^Sacrifice another creature: Exile this creature/);
  assert.equal(faces[1].oracleText, "You may sacrifice a creature rather than pay this spell's mana cost.");
});

test('an alternative cost written as "pay {Cost} rather than" answers its participle', () => {
  const [def] = mergeMechanics([], [{ keyword: 'rush', reminder: "You may pay {Cost} rather than pay this spell's mana cost." }]);
  const face = compileDraftInput({ faces: [{ name: 'A', oracleText: 'Rush {R}\nWhen this creature enters, if it was rushed, draw a card.' }] }, [def]).faces[0];
  assert.match(face.oracleText, /if its \{R\} cost was paid, draw a card/);
  assert.deepEqual(face.customKeywordPayments, ['rush']);
});

test('"you may pay {Cost} to <effect>" keeps the effect as an "If you do" consequence', () => {
  const { expansion } = validateMechanic({ keyword: 'glean', reminder: 'When this creature enters, you may pay {Cost} to draw a card.' });
  assert.equal(expansion, 'When this creature enters, you may pay {Cost}. If you do, draw a card.');
  // An alternative way to cast is not a purchased effect.
  assert.equal(validateMechanic({ keyword: 'rush', reminder: 'You may pay {Cost} to cast this spell.' }).expansion, 'You may pay {Cost} to cast this spell.');
});
