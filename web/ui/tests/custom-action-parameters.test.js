import test from 'node:test';
import assert from 'node:assert/strict';
import { validateMechanic, compileDraftInput } from '../src/lib/custom-mechanics.js';
import { expandActionParameterDeclarations } from '../src/lib/reminder-action-parameters.js';

test('again invokes one additional complete custom action program', () => {
  const def = validateMechanic({keyword:'Enlistment', reminder:'Create a 0/0 white Soldier Army creature token. Then put a +1/+1 counter on each Army you control.'});
  const original = 'When this creature enters, enlistment. Then enlistment again.';
  const face = compileDraftInput({faces:[{oracleText:original}]}, [def]).faces[0];
  assert.equal(face.displayOracleText, original);
  assert.equal(face.oracleText.match(/Create a 0\/0/g).length, 2);
  assert.equal(face.oracleText.match(/put a \+1\/\+1 counter on each Army/g).length, 2);
  assert.ok(!/enlistment|again/i.test(face.oracleText));
});

const definition = () => validateMechanic({
  keyword: 'Instruct {Object} {Ability}',
  reminder: 'If {Object} has {Ability}, put a +1/+1 counter on it. Otherwise, put a {Ability} counter on it.',
});
test('typed action operands bind inline without a mechanic-name branch', () => {
  const def = definition();
  assert.equal(expandActionParameterDeclarations('When this creature enters, instruct it flying.', def),
    'When this creature enters, If it has flying, put a +1/+1 counter on it. Otherwise, put a flying counter on it.');
  assert.equal(expandActionParameterDeclarations('• Instruct target creature double strike.', def),
    '• If target creature has double strike, put a +1/+1 counter on it. Otherwise, put a double strike counter on it.');
  assert.equal(expandActionParameterDeclarations('Instruct Mentor lifelink.', def, 'Mentor'),
    'If Mentor has lifelink, put a +1/+1 counter on it. Otherwise, put a lifelink counter on it.');
});
test('binding preserves quoted text and rejects unknown operands or nested programs', () => {
  const def = definition();
  for (const text of [
    'This creature has "Instruct it flying."',
    'Instruct it your choice of haste, menace, or vigilance. Then repeat this process X times.',
    'Instruct it flying if you control a white permanent.',
    'Instruct another player flying.',
    'Instruct it madeupability.',
    'Instruct it\nflying.',
  ]) assert.equal(expandActionParameterDeclarations(text, def), text);
});
test('custom-card expansion retains the authored display and identity', () => {
  const def = definition();
  const original = 'When this creature enters, instruct it flying.';
  const [face] = compileDraftInput({ faces: [{ name: 'Pupil', oracleText: original }] }, [def]).faces;
  assert.equal(face.displayOracleText, original);
  assert.deepEqual(face.customKeywordIdentities, ['instruct']);
  assert.match(face.oracleText, /put a flying counter on it/);
});
test('action layouts validate matching operands and require explicit layout', () => {
  assert.throws(() => validateMechanic({ keyword: 'instruct', reminder: 'Put a {Ability} counter on it.' }), /action layout/);
  assert.throws(() => validateMechanic({ keyword: 'Instruct {Object} {Ability}', reminder: 'Put a counter on {Object}.' }), /same operands/);
  assert.throws(() => validateMechanic({ keyword: 'Instruct {Object}{Ability}', reminder: '{Object} gains {Ability}.' }), /space/);
});

test('literal mana symbols are not action operands', () => {
  const def = validateMechanic({ keyword: 'Arm {Object}', reminder: 'Create a colorless Equipment artifact token with "Equipped creature gets +1/+0." and equip {2} attached to {Object}.' });
  assert.match(expandActionParameterDeclarations('Arm target creature.', def), /equip \{2\} attached to target creature\.$/);
});
test('Number and Type parameters bind complete inline declarations', () => {
  const nourish = validateMechanic({ keyword: 'Nourish {Number}', reminder: 'You gain {Number} life. Draw {Number} cards.' });
  const adjudicate = validateMechanic({ keyword: 'Adjudicate {Type}', reminder: 'Tap target {Type}. Put a stun counter on it.' });
  const [face] = compileDraftInput({ faces: [{ name: 'Inline', oracleText: 'When this creature enters, nourish 2, then adjudicate land.' }] }, [nourish, adjudicate]).faces;
  assert.equal(face.oracleText, 'When this creature enters, You gain 2 life. Draw 2 cards. Then Tap target land. Put a stun counter on it.');
  const [conditional] = compileDraftInput({ faces: [{ name: 'Conditional', oracleText: 'Adjudicate land if able.' }] }, [adjudicate]).faces;
  assert.equal(conditional.oracleText, 'Adjudicate land if able.');
});
test('fixed repetition expands the complete optional program independently', () => {
  const germinate = validateMechanic({ keyword: 'Germinate', reminder: "You may put a +1/+1 counter on a creature. If you don't, create a 1/1 green Saproling creature token." });
  const source = 'When this creature enters, germinate four times.';
  const [face] = compileDraftInput({ faces: [{ name: 'Fungal', oracleText: source }] }, [germinate]).faces;
  assert.equal((face.oracleText.match(/You may put/g) || []).length, 4);
  assert.equal((face.oracleText.match(/If you don't/g) || []).length, 4);
  assert.equal(face.displayOracleText, source);
  assert.deepEqual(face.customKeywordIdentities, ['germinate']);
});

test('typed Name operands preserve named-card text and following instructions', () => {
  const master = validateMechanic({ keyword: 'Master', template: 'of {Name}', reminder: 'When this permanent enters, search your library for a card named {Name}, reveal it, put it into your hand, then shuffle.' });
  for (const name of ["Elder's Might", 'Monument to the Thirty', '"Chandra, Torch of Defiance"', '“Dr. Julius Jumblemorph”']) {
    const source = `Master of ${name}, then draw a card.`;
    assert.equal(expandActionParameterDeclarations(source, master), `When this permanent enters, search your library for a card named ${name}, reveal it, put it into your hand, then shuffle. Then draw a card.`);
  }
  for (const source of ['Master of Frost Breath if you control a Wizard.', 'Master of Frost Breath\nFlying', 'This creature has "Master of Frost Breath."']) {
    const expected = source === 'Master of Frost Breath\nFlying'
      ? 'When this permanent enters, search your library for a card named Frost Breath, reveal it, put it into your hand, then shuffle\nFlying'
      : source;
    assert.equal(expandActionParameterDeclarations(source, master), expected);
  }
});


test('inline variable declarations bind the full action before its fallback', () => {
  const nourish = validateMechanic({ keyword: 'Nourish {Number}', reminder: "You may put {Number} +1/+1 counters on this creature. If you don't, create {Number} 1/1 white Citizen creature tokens." });
  const [face] = compileDraftInput({ faces: [{ name: 'Spirit', oracleText: 'When this creature enters, nourish X, where X is the number of creatures you control.' }] }, [nourish]).faces;
  assert.equal(face.oracleText, "When this creature enters, You may put X +1/+1 counters on this creature, where X is the number of creatures you control. If you don't, create X 1/1 white Citizen creature tokens.");
  assert.ok(!face.oracleText.includes('., where'));
});

test('single-action player carriers preserve the authored actor and optionality', () => {
  const profit = validateMechanic({ keyword: 'Profit', reminder: 'Create a Gold token.' });
  const source = 'Exile target creature. Its controller may profit.';
  const [face] = compileDraftInput({ faces: [{ name: 'Golden', oracleText: source }] }, [profit]).faces;
  assert.equal(face.oracleText, 'Exile target creature. Its controller may Create a Gold token.');
  assert.deepEqual(face.customKeywordIdentities, ['profit']);
  for (const text of ['If you profit, draw a card.', 'This creature has "Its controller may profit."']) {
    const [unchanged] = compileDraftInput({ faces: [{ name: 'Quoted', oracleText: text }] }, [profit]).faces;
    assert.equal(unchanged.oracleText, text);
  }
  const compound = validateMechanic({ keyword: 'Profit', reminder: 'Create a Gold token. Draw a card.' });
  const [pending] = compileDraftInput({ faces: [{ name: 'Compound', oracleText: source }] }, [compound]).faces;
  assert.equal(pending.oracleText, source, 'a multi-sentence body needs explicit actor/program scope');
});

test('target preludes declare the recipient before keyword predicates', () => {
  const teach = validateMechanic({ keyword: 'Instruct {Object} {Ability}', reminder: 'Choose {Object}. If it has {Ability}, put a +1/+1 counter on it. Otherwise, put a {Ability} counter on it.' });
  assert.equal(expandActionParameterDeclarations('Instruct target creature you control flying.', teach), 'Choose target creature you control. If it has flying, put a +1/+1 counter on it. Otherwise, put a flying counter on it.');
  assert.equal(expandActionParameterDeclarations('Instruct this creature flying.', teach), 'Choose this creature. If it has flying, put a +1/+1 counter on it. Otherwise, put a flying counter on it.');
  assert.equal(teach.objectBinding, undefined, 'the complete target prelude is visible in the reminder');
});

test('invalid compound action operands do not claim a custom identity', () => {
  const teach = validateMechanic({ keyword: 'Instruct {Object} {Ability}', reminder: 'Choose {Object}. If it has {Ability}, put a +1/+1 counter on it. Otherwise, put a {Ability} counter on it.' });
  const source = 'Whenever a creature enters, instruct it flying if a creature you control has flying.';
  const [pending] = compileDraftInput({ faces: [{ name: 'Genome', oracleText: source }] }, [teach]).faces;
  assert.equal(pending.oracleText, source);
  assert.equal(pending.customKeywordIdentities, undefined);
  assert.equal(expandActionParameterDeclarations('Instruct target attacking creature with lesser power flying.', teach), 'Choose target attacking creature with lesser power. If it has flying, put a +1/+1 counter on it. Otherwise, put a flying counter on it.');
});


test('a following then instruction stays outside the expanded false arm', () => {
  const definition = validateMechanic({ keyword: 'Instruct {Object} {Ability}', reminder: 'Choose {Object}. If it has {Ability}, put a +1/+1 counter on it. Otherwise, put a {Ability} counter on it.' });
  assert.equal(expandActionParameterDeclarations('Instruct target creature flying, then draw a card.', definition), 'Choose target creature. If it has flying, put a +1/+1 counter on it. Otherwise, put a flying counter on it. Then draw a card.');
});
