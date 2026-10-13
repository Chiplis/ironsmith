import test from 'node:test';
import assert from 'node:assert/strict';
import { MsePackageStore } from '../src/lib/mse/packages.mjs';
import { loadMseTemplate, MseTemplateSet, MseTemplateContext } from '../src/lib/mse/template.mjs';
import { text } from '../src/lib/mse/script-values.mjs';

async function template(extra = '') {
  const files = {
    'example.mse-style/style': 'game: example\ncard width: 375\ncard height: 523\n',
    'example.mse-game/game': `card field:
\ttype: text
\tname: name
card field:
\ttype: text
\tname: counterpart
\tdefault: if get_back_face(card) != nil then get_back_face(card).name else "none"
card link:
\tselected: Front Face
\tlinked: Back Face
\tlocalized linked:
\t\tes: Reverso
${extra}`,
  };
  return loadMseTemplate(new MsePackageStore({ readText: async path => files[path.key] }), 'example.mse-style');
}

test('linked faces resolve UID and card inputs and expose every card in set order', async () => {
  const t = await template();
  const set = new MseTemplateSet(t, { cards: [
    { uid: 'front', card: { name: 'Day', linked_card_1: 'back', linked_relation_1: 'Back Face' } },
    { uid: 'back', card: { name: 'Night', linked_card_1: 'front', linked_relation_1: 'Front Face' } },
  ] }).initialize();
  const [front, back] = set.contexts;
  assert.equal(text(front.field('card', 'counterpart')), 'Night');
  assert.equal(front.runtime.run('get_back_face(card) == get_card_from_uid("back")'), true);
  assert.equal(front.runtime.run('get_card_from_uid("front") == get_card_from_uid("back")'), false);
  assert.equal(text(back.runtime.run('get_front_face("back").name')), 'Day');
  assert.equal(front.runtime.run('length(set.cards)'), 2);
  assert.equal(front.runtime.run('set.cards.0.uid'), 'front');
  assert.equal(front.runtime.run('get_back_face("missing")'), null);
  assert.equal(front.runtime.run('get_front_face(card)'), null);
  assert.equal(front.runtime.run('get_cards_from_link(card, linked_relation:"Reverso").0.uid'), 'back');
  assert.equal(front.runtime.run('type_name(card)'), 'card');
  assert.deepEqual(front.runtime.run('get_link_from_cards(card, linked_card:"back")').values, ['Front Face', 'Back Face']);
});

test('adding and removing links updates both cards and invalidates derived fields', async () => {
  const set = new MseTemplateSet(await template(), { cards: [{ uid: 'a', card: { name: 'A' } }, { uid: 'b', card: { name: 'B' } }] }).initialize();
  const [a, b] = set.contexts;
  assert.equal(text(a.field('card', 'counterpart')), 'none');
  assert.equal(a.runtime.run('add_link(card, linked_card:"b", selected_relation:"Front Face", linked_relation:"Back Face") == get_card_from_uid("b")'), true);
  assert.equal(a.runtime.run('card.linked_card_1'), 'b');
  assert.equal(b.runtime.run('card.linked_relation_1'), 'Front Face');
  assert.equal(text(a.field('card', 'counterpart')), 'B');
  assert.equal(a.runtime.run('has_link(card, linked_relation:"Back Face")'), true);
  assert.equal(a.runtime.run('remove_links(card, linked_relation:"Back Face").0.uid'), 'b');
  assert.equal(text(a.field('card', 'counterpart')), 'none');
  assert.equal(b.runtime.run('get_front_face(card)'), null);
});

test('game face scripts execute with the requested card context', async () => {
  const t = await template('get back face script: if card.name == "A" then get_card_from_uid("b") else nil\n');
  const set = new MseTemplateSet(t, { cards: [{ uid: 'a', card: { name: 'A' } }, { uid: 'b', card: { name: 'B' } }] }).initialize();
  assert.equal(set.contexts[1].runtime.run('get_back_face("a").uid'), 'b');
  assert.equal(set.contexts[0].runtime.run('get_back_face("b")'), null);
  assert.throws(() => new MseTemplateSet(t, { cards: [{ uid: 'same' }, { uid: 'same' }] }), { code: 'card-uid' });
  assert.equal(new MseTemplateContext(t).initialize().runtime.run('length(set.cards)'), 1);
});

test('set counts, sorting and collector positions evaluate each card in its own context', async () => {
  const set = new MseTemplateSet(await template(), { cards: [
    { uid: 'b', card: { name: 'B', notes: 'exclude' } },
    { uid: 'c', card: { name: 'C' } },
    { uid: 'a', card: { name: 'A' } },
  ] }).initialize();
  const runtime = set.contexts[0].runtime;
  assert.equal(runtime.run('type_name(set)'), 'set');
  assert.equal(runtime.run('length(set, filter:{card.notes == ""})'), 2);
  assert.equal(runtime.run('number_of_items(in:set, filter:{card.name != "A"})'), 2);
  assert.equal(runtime.run('position(of:get_card_from_uid("c"), in:set, order_by:{card.name})'), 2);
  assert.equal(runtime.run('position(of:get_card_from_uid("c"), in:set, order_by:{card.name}, filter:{card.notes == ""})'), 1);
  assert.equal(runtime.run('position(of:card, in:set, order_by:{card.name}, filter:{card.notes == ""})'), -1);
  assert.equal(runtime.run('sort_list(set, order_by:{card.name}).0.uid'), 'a');
  assert.equal(runtime.run('for each c in set do c.uid'), 'bca');
});
