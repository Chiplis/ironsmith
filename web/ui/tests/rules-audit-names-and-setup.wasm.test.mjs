import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import test from 'node:test';
import {cardNameKey, cardRouteKey} from '../src/lib/card-asset-cache.js';
import {finishPuzzlePregame, passToFirstMain} from './fixtures/native-game-setup.mjs';

const pkg = new URL(process.env.IRONSMITH_TEST_ENGINE || '../../wasm_demo/pkg/engine.js', import.meta.url);
const {default: init, WasmGame} = await import(pkg.href);
await init({module_or_path: await readFile(new URL('engine_bg.wasm', pkg))});
const assets = new URL('../public/cards/', import.meta.url);
async function readPackage(route) {
  return JSON.parse(await readFile(new URL(route + '.json', assets), 'utf8'));
}
async function register(game, names) {
  const packages = await Promise.all(names.map(name => readPackage(cardRouteKey(name))));
  assert.deepEqual(JSON.parse(game.registerExternalCardSourcesJson(JSON.stringify(packages))).failed, []);
}

test('the derived route for Index retrieves and registers its canonical card package', async () => {
  const manifest = await readPackage('index');
  const route = manifest.cards.find(card => card.name === 'Index').route;
  assert.equal(cardRouteKey('Index'), route);
  assert.equal(cardNameKey('Index'), 'index');
  const card = await readPackage(route);
  assert.equal(card.canonicalName, 'Index');
  const game = new WasmGame();
  try {
    assert.deepEqual(JSON.parse(game.registerExternalCardSourcesJson(JSON.stringify([card]))), {loaded:1, failed:[]});
    assert.ok(game.isKnownCardName('Index'));
    const embeddedIndex = game.getEmbeddedCardCatalogIndexJson();
    assert.ok(embeddedIndex, 'the packaged engine contains the card catalog');
    const embeddedRoute = JSON.parse(embeddedIndex).cards.find(c => c.name === 'Index').route;
    assert.equal(embeddedRoute, route);
    assert.equal(JSON.parse(game.getEmbeddedCardSourceJson(embeddedRoute)).canonicalName, 'Index');
  } finally { game.free(); }
});

test('a companion accepts a Time Lord Doctor and rejects a Time Lord with a different creature type', async () => {
  const game = new WasmGame();
  try {
    await register(game, ['The Fourth Doctor', 'Sarah Jane Smith', 'The Master, Multiplied', 'Wastes', 'Plains', 'Isamaru, Hound of Konda']);
    const start = doctor => game.startMatch({
      playerNames:['Alice','Bob'], startingLife:40, seed:1, format:'commander',
      decks:[Array(98).fill('Wastes'), Array(99).fill('Plains')],
      commanders:[[doctor, 'Sarah Jane Smith'], ['Isamaru, Hound of Konda']],
    });
    assert.throws(() => start('The Master, Multiplied'), /partner/i);
    assert.doesNotThrow(() => start('The Fourth Doctor'));
  } finally { game.free(); }
});

test('Meddling Mage requires a legal individual name and accepts a corrected answer to the same prompt', async () => {
  const game = new WasmGame();
  try {
    game.resetEmpty(['Alice','Bob'],20);
    await register(game, ['Meddling Mage','Omniscience','Fire','Forest','Mountain']);
    game.addCardToZone(0,'Omniscience','battlefield',true);
    const mage = game.addCardToZone(0,'Meddling Mage','hand',true);
    const split = game.addCardToZone(0,'Fire','hand',true);
    finishPuzzlePregame(game);
    let s = passToFirstMain(game);
    const action = s.decision.actions.find(a => a.kind === 'cast_spell' && Number(a.object_id) === Number(mage));
    s = game.dispatch({type:'priority_action',action_ref:action.action_ref});
    while (s.decision.kind === 'priority') {
      s = game.dispatch({type:'priority_action',action_ref:{kind:'pass_priority'}});
    }
    for (const invalid of ['Forest', 'Fire // Ice']) {
      assert.equal(game.uiState().decision.kind, 'text_input');
      assert.equal(game.isLegalCardNameChoice(invalid), false);
      assert.throws(() => game.dispatch({type:'text_choice',value:invalid}), /card name/);
      assert.ok(!game.uiState().players[0].battlefield.some(c => c.name === 'Meddling Mage'));
    }
    assert.equal(game.isLegalCardNameChoice('Ice'), true);
    s = game.dispatch({type:'text_choice',value:'Ice'});
    assert.ok(s.players[0].battlefield.some(c => c.name === 'Meddling Mage'));
    const castable = s.decision.actions.filter(a => a.kind === 'cast_spell' && Number(a.object_id) === Number(split));
    assert.ok(castable.some(a => a.label.startsWith('Cast Fire')));
    assert.ok(!castable.some(a => a.label.startsWith('Cast Ice')));
  } finally { game.free(); }
});
