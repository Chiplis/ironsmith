import test from 'node:test';
import assert from 'node:assert/strict';
import { interactiveRulesView } from '../src/lib/inspector-ability-lines.js';
import { groupManaAbilities } from '../src/lib/group-mana-abilities.js';
const green = { index: 5, ability_text: '{T}: Add {G}.' };
const white = { index: 9, ability_text: '{T}: Add {W}.' };
const text = '{T}: Add {G}.\n{T}: Add {W}.';
const group = (source, displayed = source, actions = [], locale = 'en') => groupManaAbilities(interactiveRulesView(source, displayed, actions), locale);

test('identical costs combine without losing action identities', () => {
  const view = group(text, text, [white, green]);
  assert.deepEqual(view.lines, ['{T}: Add {G} or {W}.']);
  assert.deepEqual(view.manaGroups.get(0).options.map(o => o.actions[0]), [green, white]);
  assert.deepEqual(view.sourceLines, [[green.ability_text, white.ability_text]]);
});

test('all mana choices remain visible when only one is available', () => {
  const view = group(text, text, [white]);
  assert.deepEqual(view.lines, ['{T}: Add {G} or {W}.']);
  assert.deepEqual(view.manaGroups.get(0).options.map(o => o.actions), [[], [white]]);
  assert.deepEqual(group(text).lines, view.lines);
});

test('translated lines group with localized disjunctions and output actions', () => {
  for (const [locale, translated, expected] of [
    ['es', '{T}: Agrega {G}.\n{T}: Agrega {W}.', '{T}: Agrega {G} o {W}.'],
    ['fr', '{T} : Ajoutez {G}.\n{T} : Ajoutez {W}.', '{T} : Ajoutez {G} ou {W}.'],
    ['ja', '{T}：{G}を加える。\n{T}：{W}を加える。', '{T}：{G}または{W}を加える。'],
  ]) {
    const view = group(text, translated, [white], locale);
    assert.deepEqual(view.lines, [expected]);
    assert.equal(view.manaGroups.get(0).options[1].actions[0], white);
  }
});

test('different costs, restrictions, and extra effects stay separate', () => {
  for (const source of [
    '{T}: Add {G}.\n{1}, {T}: Add {W}.',
    '{T}, Pay 1 life: Add {G}.\n{T}, Pay 2 life: Add {W}.',
    '{T}: Add {G}.\n{T}: Add {W}. Spend this mana only to cast creatures.',
    '{T}: Add {G}. You gain 1 life.\n{T}: Add {W}. You gain 1 life.',
    '{T}: Add {G} for each creature you control.\n{T}: Add {W}.',
  ]) assert.equal(group(source).lines.length, 2, source);
});

test('missing translations keep the entire combined sentence in English', () => {
  assert.deepEqual(group(text, text, [], 'es').lines, ['{T}: Add {G} or {W}.']);
});

test('identical complex costs and fixed multi-mana outputs combine', () => {
  assert.deepEqual(group('{1}, {T}, Pay 1 life: Add {G}{G}.\n{1}, {T}, Pay 1 life: Add {W}{W}.').lines,
    ['{1}, {T}, Pay 1 life: Add {G}{G} or {W}{W}.']);
});

test('three colors and colorless retain independent choices', () => {
  const view = group('{T}: Add {C}.\n{T}: Add {G}.\n{T}: Add {W}.');
  assert.deepEqual(view.lines, ['{T}: Add {C}, {G}, or {W}.']);
  assert.equal(view.manaGroups.get(0).options.length, 3);
});

test('parenthesized basic-land abilities group without dropping parentheses', () => {
  assert.deepEqual(group('({T}: Add {G}.)\n({T}: Add {W}.)').lines, ['({T}: Add {G} or {W}.)']);
});

test('unrelated lines retain their action and index after grouping', () => {
  const draw = { ability_text: '{2}: Draw a card.' };
  const source = text + '\n{2}: Draw a card.';
  const view = group(source, source, [green, white, draw]);
  assert.deepEqual(view.lines, ['{T}: Add {G} or {W}.', draw.ability_text]);
  assert.equal(view.actions.get(1)[0], draw);
});
