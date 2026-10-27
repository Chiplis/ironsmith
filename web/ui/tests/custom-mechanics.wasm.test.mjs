import test from 'node:test';
import assert from 'node:assert/strict';
import { initWasmGame } from '../../../scripts/wasm-test-harness.mjs';
import { finishPuzzlePregame, passToFirstMain } from './fixtures/native-game-setup.mjs';
import { prepareCustomCard } from '../src/lib/custom-mechanics.js';
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
