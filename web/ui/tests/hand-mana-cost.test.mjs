import test from 'node:test';
import assert from 'node:assert/strict';
import {handManaCostPresentation} from '../src/lib/hand-mana-cost.js';
import {cardGlintFace} from '../src/lib/card-glint/card.mjs';
import {cardFrameSnapshot} from '../src/lib/card-frame-snapshot.mjs';
const card = {id:7,name:'Cost Preview',mana_cost:'{2}{U}',type_line:'Instant'};
const state = mana_cost => ({players:[{hand_mana_costs:[{id:7,name:card.name,mana_cost}]}]});

test('live reductions, increases and zero update only the displayed mana symbols', () => {
  for (const [cost, symbols] of [['{U}',['U']], ['{3}{U}',['3','U']], ['{0}',['0']]]) {
    const shown = handManaCostPresentation(card, state(cost));
    const snapshot = JSON.parse(cardFrameSnapshot(shown));
    const frame = cardGlintFace(snapshot);
    assert.equal(snapshot.mana_cost, '{2}{U}');
    assert.equal(snapshot.display_mana_cost, cost);
    assert.deepEqual(frame.manaSymbols, symbols);
    assert.deepEqual(frame.colors, ['U'], 'reductions do not recolor the printed frame');
  }
  assert.equal(card.mana_cost, '{2}{U}');
  assert.equal(card.display_mana_cost, undefined);
});

test('missing prices restore printed cost and a different hovered face never inherits its front price', () => {
  assert.equal(handManaCostPresentation(card, null), card);
  assert.equal(handManaCostPresentation(card, {players:[]}), card);
  assert.equal(handManaCostPresentation({...card,name:'Other face'},state('{U}')).display_mana_cost, undefined);
  const paired = {...card,layout:'modal_dfc',card_faces:[{name:card.name,mana_cost:'{2}{U}'},{name:'Back',mana_cost:'{G}'}]};
  const shown = handManaCostPresentation(paired,state('{U}'));
  assert.deepEqual(cardGlintFace(shown,{faceIndex:0}).manaSymbols,['U']);
  assert.deepEqual(cardGlintFace(shown,{faceIndex:1}).manaSymbols,['G']);
  assert.equal(paired.card_faces[0].display_mana_cost,undefined);
  assert.equal(handManaCostPresentation(card,state('{2}{U}')),card,'unchanged cost reuses printed presentation');
});

test('split cards without a matching normal face keep their individual printed costs', () => {
  const split = {...card,name:'Left // Right',layout:'split',card_faces:[{name:'Left',mana_cost:'{U}'},{name:'Right',mana_cost:'{R}'}]};
  const shown = handManaCostPresentation(split,{players:[{hand_mana_costs:[{id:7,name:split.name,mana_cost:'{0}'}]}]});
  assert.equal(shown,split);
});

test('deferred prices merge only into their current snapshot and visible hand', async () => {
  const {mergeHandManaCosts} = await import('../src/lib/hand-mana-cost-analysis.js');
  const current = {snapshot_id:12,perspective:0,players:[{id:0,hand_cards:[card],hand_mana_costs:[]},{id:1,hand_cards:[],hand_mana_costs:[]}]};
  const preview = {snapshot_id:12,perspective:0,prices:[{id:7,name:card.name,mana_cost:'{U}'},{id:9,name:'Hidden',mana_cost:'{R}'}]};
  assert.equal(handManaCostPresentation(card,current),card,'printed price remains usable before results arrive');
  const next = mergeHandManaCosts(current,preview);
  assert.equal(handManaCostPresentation(card,next).display_mana_cost,'{U}');
  assert.deepEqual(next.players[1].hand_mana_costs,[]);
  assert.equal(current.players[0].hand_mana_costs.length,0,'published snapshots are immutable');
  for (const result of [null,{...preview,snapshot_id:11},{...preview,perspective:1},{...preview,prices:null}]) {
    assert.equal(mergeHandManaCosts(current,result),current,'late, failed, or other-perspective results are ignored');
  }
});
