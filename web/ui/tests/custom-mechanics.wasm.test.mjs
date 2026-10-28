import test from 'node:test';
import assert from 'node:assert/strict';
import { initWasmGame } from '../../../scripts/wasm-test-harness.mjs';
import { finishPuzzlePregame, passToFirstMain } from './fixtures/native-game-setup.mjs';
import { prepareCustomCard, probeMechanic } from '../src/lib/custom-mechanics.js';
import { REMINDER_TEMPLATES } from '../src/lib/mechanics-catalog.js';
import { variukDraft } from './custom-mechanics-fixture.mjs';

test('arbitrary custom names expand different reminders with inline and spaced placement', {timeout: 120000}, async () => {
  const { game } = await initWasmGame({ pkg: 'demo' });
  try {
    for (const [keyword, reminder] of [
      ['glimmer', 'Draw a card.'],
      ['emberwake', 'You gain 2 life.'],
      ['moonpulse', 'Create a Treasure token.'],
    ]) {
      for (const separator of [' ', '\n', '\n\n']) {
        const draft = { faces: [{ name: 'Custom Reminder Test', cardTypes: ['Instant'], manaCost: '{0}',
          oracleText: `${keyword}.${separator}(${reminder})`,
        }] };
        const prepared = await prepareCustomCard(draft, input => game.previewCustomCard(input));
        assert.equal(prepared.preview.canCreate, true);
        assert.equal(prepared.input.faces[0].oracleText, reminder);
        assert.equal(prepared.draft.customMechanics[0].keyword, keyword);
        assert.equal(prepared.input.faces[0].displayOracleText, `${keyword}.`);
      }
    }
  } finally { game.free(); }
});

test('discovers a reminder-defined action behind line-family and target diagnostics', {timeout: 120000}, async () => {
  const { game } = await initWasmGame({ pkg: 'demo' });
  try {
    for (const [name, cardTypes] of [['Custom Card', ['Creature']], ['Eerie Static', ['Instant']]]) {
      const draft = { faces: [{
        name, cardTypes, manaCost: '{0}', power: '2', toughness: '2',
        oracleText: 'Eerie Static deals 2 damage to any target, shift. (Noncreature, nonartifact spells cost {1} less to cast until end of turn.)',
      }] };
      let calls = 0;
      const prepared = await prepareCustomCard(draft, input => {
        calls++;
        return game.previewCustomCard(input);
      });
      assert.equal(calls, 2);
      assert.equal(prepared.preview.canCreate, true);
      assert.equal(prepared.draft.customMechanics[0].keyword, 'shift');
      assert.match(prepared.input.faces[0].oracleText, /nonartifact spells cost \{1\} less/);
      assert.match(prepared.input.faces[0].displayOracleText, /, shift\./);
    }
  } finally { game.free(); }
});

test('Variuk retains shift in presentation while its real trigger stacks reductions and adds counters', {timeout: 120000}, async () => {
  const {game} = await initWasmGame({pkg:'demo'});
  try {
    game.resetEmpty(['Alice','Bob'],20);
    const prepared = await prepareCustomCard(variukDraft(), input => game.previewCustomCard(input));
    assert.match(prepared.preview.faces[0].compiledText.join('\n'), /shift, then put/);
    assert.ok(!prepared.preview.faces[0].compiledText.join('\n').includes('nonartifact'));
    const variuk = game.createCustomCard({draft:prepared.input,playerIndex:0,zoneName:'battlefield',skipTriggers:true});
    const add = (name, cardTypes, oracleText = '', manaCost = '{2}', zoneName = 'hand', playerIndex = 0, size = '2') => game.createCustomCard({
      draft:{faces:[{name,cardTypes,oracleText,manaCost,...(cardTypes.includes('Creature') ? {power:size,toughness:size} : {})}]},
      playerIndex,zoneName,skipTriggers:true,
    });
    const small = add('Small Ally',['Creature'],'','{0}','battlefield');
    const equal = add('Equal Ally',['Creature'],'','{0}','battlefield',0,'4');
    const opponent = add('Opponent Creature',['Creature'],'','{0}','battlefield',1);
    const first = add('First Touch',['Instant'],'Target creature gains vigilance until end of turn.','{0}');
    const second = add('Second Touch',['Instant'],'Target creature gains vigilance until end of turn.','{0}');
    const sorcery = add('Matching Sorcery',['Sorcery'],'You gain 1 life.');
    const enchantment = add('Matching Enchantment',['Enchantment']);
    const colored = add('Colored Instant',['Instant'],'You gain 1 life.','{1}{U}');
    const artifact = add('Excluded Artifact',['Artifact']);
    const creature = add('Excluded Creature',['Creature']);
    const artifactCreature = add('Excluded Artifact Creature',['Artifact','Creature']);
    const opponentSpell = add('Opponent Spell',['Sorcery'],'You gain 1 life.','{2}','hand',1);
    finishPuzzlePregame(game);
    passToFirstMain(game);
    const prices = (seat = 0) => {
      game.setPerspective(seat);
      const state = game.uiState();
      return new Map(game.previewHandManaCosts(BigInt(state.snapshot_id)).prices.map(price => [Number(price.id),price.mana_cost]));
    };
    assert.equal(prices().get(Number(sorcery)), '{2}');
    for (const [index, spell] of [first,second].entries()) {
      const state = game.uiState();
      const action = state.decision.actions.find(action => action.kind === 'cast_spell' && Number(action.object_id) === Number(spell));
      assert.ok(action, 'the free targeting spell is castable');
      const targeting = game.dispatch({type:'priority_action',action_ref:action.action_ref});
      assert.equal(targeting.decision.kind, 'targets');
      let resolving = game.dispatch({type:'select_targets',targets:[{kind:'object',object:Number(variuk)}]});
      assert.equal(resolving.stack_objects.length, 2, 'targeting Variuk puts its trigger over the spell');
      assert.match(resolving.stack_objects[0].ability_text, /shift/i);
      for(let pass = 0; resolving.stack_objects.length && pass < 10; pass++) {
        resolving = game.dispatch({type:'priority_action',action_ref:{kind:'pass_priority'}});
      }
      assert.equal(resolving.stack_objects.length, 0);
      assert.equal(game.objectDetails(small).power, 3 + index);
      assert.equal(game.objectDetails(equal).power, 4, 'equal power is excluded');
      assert.equal(game.objectDetails(variuk).power, 4, 'Variuk does not put a counter on itself');
      assert.equal(game.objectDetails(opponent).power, 2, 'opponents creatures are excluded');
      const costs = prices();
      assert.equal(costs.get(Number(sorcery)), index === 0 ? '{1}' : '{0}');
      assert.equal(costs.get(Number(enchantment)), index === 0 ? '{1}' : '{0}');
      assert.equal(costs.get(Number(colored)), '{U}', 'generic reductions do not pay colored mana');
      for (const id of [artifact,creature,artifactCreature]) assert.equal(costs.get(Number(id)), '{2}');
      assert.equal(prices(1).get(Number(opponentSpell)), '{2}');
      game.setPerspective(0);
    }
    let state = game.uiState();
    for (let step=0; state.turn_number < 2 && step < 100; step++) {
      const kind = state.decision?.kind;
      const command = kind === 'priority' ? {type:'priority_action',action_ref:{kind:'pass_priority'}}
        : kind === 'attackers' ? {type:'declare_attackers',declarations:[]}
        : kind === 'blockers' ? {type:'declare_blockers',declarations:[]} : null;
      assert.ok(command, `Unexpected end-turn decision: ${JSON.stringify(state.decision)}`);
      state = game.dispatch(command);
    }
    assert.ok(state.turn_number >= 2);
    assert.equal(prices().get(Number(sorcery)), '{2}', 'shift expires at end of turn');
    assert.equal(prices().get(Number(colored)), '{1}{U}');
  } finally { game.free(); }
});

test('a standalone mechanic compiles its reminder on a test card and reports compiler errors', {timeout: 120000}, async () => {
  const { game } = await initWasmGame({ pkg: 'demo' });
  const compile = input => game.previewCustomCard(input);
  try {
    assert.deepEqual(await probeMechanic({ keyword: 'glimmer', reminder: 'Draw a card.' }, compile), { checked: true, usage: 'Glimmer.', permanentAbility: false });
    assert.deepEqual(await probeMechanic({ keyword: 'blink', reminder: '{Cost}: Exile this creature, then return it to the battlefield under its owner\'s control.' }, compile), { checked: true, usage: 'Blink {1}', permanentAbility: true });
    // A "may pay" sentence is a spell instruction: dead text on a permanent.
    assert.equal((await probeMechanic({ keyword: 'blink', reminder: 'You may pay {Cost} to exile it and return it to the battlefield under your control.' }, compile)).permanentAbility, false);
    await assert.rejects(probeMechanic({ keyword: 'bogus', reminder: 'Florp the wibble twice daily.' }, compile),
      error => error.usage === 'Bogus.' && /Florp the wibble/.test(error.message));
  } finally { game.free(); }
});

test('every Mechanics-tab template compiles to its kind of rules text', {timeout: 120000}, async () => {
  const { game } = await initWasmGame({ pkg: 'demo' });
  const expected = {
    'Static ability': raw => (raw.match(/kind: Static\(/g) || []).length > 1,
    'Activated ability': raw => /kind: Activated\(/.test(raw),
    'Triggered ability': raw => /kind: Triggered\(/.test(raw),
    'Replacement effect': raw => /EventReplacementWithEffects/.test(raw),
    'Additional cost': raw => /optional_costs: \[\s*\S/.test(raw),
    'Alternative cost': raw => /alternative_casts: \[\s*\S/.test(raw),
    'Spell instruction': raw => /spell_effect: Some/.test(raw),
  };
  try {
    assert.deepEqual(REMINDER_TEMPLATES.map(template => template.kind), Object.keys(expected));
    for (const template of REMINDER_TEMPLATES) {
      const cardTypes = template.kind === 'Spell instruction' ? ['Instant'] : ['Creature'];
      const draft = { faces: [{ name: 'Template Card', cardTypes, manaCost: '{1}', power: '2', toughness: '2', oracleText: template.usage }],
        customMechanics: [{ keyword: template.keyword, reminder: template.reminder, sourceCard: 'the Mechanics tab' }] };
      const prepared = await prepareCustomCard(draft, input => game.previewCustomCard(input));
      assert.doesNotMatch(prepared.input.faces[0].oracleText, /\{Cost\}/i, template.kind);
      assert.ok(expected[template.kind](prepared.preview.faces[0].rawCompilation), template.kind);
    }
    // The cost templates' notes promise a participle check; it compiles.
    for (const [kind, usage] of [['Additional cost', 'Empower {2}'], ['Alternative cost', 'Rush {R}']]) {
      const template = REMINDER_TEMPLATES.find(candidate => candidate.kind === kind);
      const participle = template.note.match(/"if it was ([a-z]+)"/)[1];
      const draft = { faces: [{ name: 'Template Card', cardTypes: ['Creature'], manaCost: '{1}', power: '2', toughness: '2',
        oracleText: `${usage}\nWhen this creature enters, if it was ${participle}, draw a card.` }],
        customMechanics: [{ keyword: template.keyword, reminder: template.reminder, sourceCard: 'the Mechanics tab' }] };
      const prepared = await prepareCustomCard(draft, input => game.previewCustomCard(input));
      assert.match(prepared.preview.faces[0].rawCompilation, /kind: Triggered\(/, kind);
    }
  } finally { game.free(); }
});
