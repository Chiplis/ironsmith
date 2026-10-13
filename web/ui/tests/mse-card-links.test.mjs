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

test('card dates preserve supplied metadata and update modification time on edits',async()=>{
  const {MseCardRegistry}=await import('../src/lib/mse/card-links.mjs');
  const t=await template('card field:\n\ttype: text\n\tname: changed_at\n\tdefault: to_string(card.time_modified)\n');
  let now=new Date(2025,0,2,3,4,5);
  const registry=new MseCardRegistry({clock:()=>now});
  const context=new MseTemplateContext(t,{cardRegistry:registry,fieldUpdateMode:'ordered',card:{time_created:'2020-06-07 08:09:10'}}).initialize();
  assert.equal(context.runtime.run('type_name(card.time_created)'),'date');
  assert.equal(context.runtime.run('to_string(card.time_created)'),'2020-06-07 08:09:10');
  assert.equal(text(context.field('card','changed_at')),'2025-01-02 03:04:05');
  now=new Date(2025,0,3,4,5,6);context.setField('card','name','Edited');
  assert.equal(text(context.field('card','changed_at')),'2025-01-03 04:05:06');
  assert.equal(context.runtime.run('to_string(card.time_created)'),'2020-06-07 08:09:10');
});

test('card styling helpers return the selected card context and reject non-card input',async()=>{
  const t=await template();
  const set=new MseTemplateSet(t,{cards:[{uid:'a',styling:{theme:'white'}},{uid:'b',styling:{theme:'black'}}]}).initialize();
  const runtime=set.contexts[0].runtime;
  assert.equal(text(runtime.run('get_card_styling(card).theme')),'white');
  assert.equal(text(runtime.run('get_card_styling(get_card_from_uid("b")).theme')),'black');
  assert.equal(runtime.run('get_card_stylesheet(card).name'),'example.mse-style');
  assert.throws(()=>runtime.run('get_card_styling("b")'),{code:'card-styling'});
});

test('ordered field dependencies propagate across cards in the same set',async()=>{
  const t=await template('card field:\n\ttype: text\n\tname: summary\n\tdefault: set.cards.0.name + "/" + set.cards.1.name\n');
  const set=new MseTemplateSet(t,{fieldUpdateMode:'ordered',cards:[{card:{name:'A'}},{card:{name:'B'}}]}).initialize();
  assert.deepEqual(set.contexts.map(c=>text(c.field('card','summary'))),['A/B','A/B']);
  set.contexts[1].setField('card','name','C');
  assert.deepEqual(set.contexts.map(c=>text(c.field('card','summary'))),['A/C','A/C']);
});

test('ordered link edits refresh face lookups, raw slots, relation helpers and downstream fields',async()=>{
  const t=await template(`card field:
\ttype: text
\tname: raw_link
\tdefault: card.linked_card_1 + "/" + card.linked_relation_1
card field:
\ttype: text
\tname: linked
\tdefault: to_string(has_link(card, linked_relation:"Back Face"))
card field:
\ttype: text
\tname: summary
\tdefault: card.counterpart + ":" + card.linked
card field:
\ttype: text
\tname: untouched
\tscript: input + "!"
`);
  const set=new MseTemplateSet(t,{fieldUpdateMode:'ordered',cards:[{uid:'a',card:{name:'A'}},{uid:'b',card:{name:'B'}}]}).initialize();
  const [a,b]=set.contexts;
  assert.equal(text(a.field('card','summary')),'none:false');
  a.runtime.run('add_link(card, linked_card:"b", selected_relation:"Front Face", linked_relation:"Back Face")');
  assert.equal(text(a.field('card','summary')),'B:true');
  assert.equal(text(a.field('card','raw_link')),'b/Back Face');
  assert.equal(text(b.field('card','raw_link')),'a/Front Face');
  b.setField('card','name','Changed');
  assert.equal(text(a.field('card','summary')),'Changed:true');
  a.runtime.run('remove_links(card, linked_card:"b")');
  assert.equal(text(a.field('card','summary')),'none:false');
  assert.equal(text(a.field('card','raw_link')),'/');
  assert.equal(text(b.field('card','raw_link')),'/');
  assert.deepEqual(set.contexts.map(c=>text(c.field('card','untouched'))),['!','!']);
});

test('link-mutating field scripts do not recursively start ordered updates',async()=>{
  const t=await template('card field:\n\ttype: text\n\tname: create_link\n\tdefault: if card.name == "A" then add_link(card, linked_card:"b", selected_relation:"Front Face", linked_relation:"Back Face") else nil\n');
  const set=new MseTemplateSet(t,{fieldUpdateMode:'ordered',cards:[{uid:'a',card:{name:'A'}},{uid:'b',card:{name:'B'}}]}).initialize();
  const [a,b]=set.contexts;
  assert.equal(a.runtime.run('get_back_face(card).uid'),'b');
  assert.equal(b.runtime.run('get_front_face(card).uid'),'a');
  assert.equal(set.registry.fieldUpdates.running,false);
  a.runtime.run('remove_links(card, linked_card:"b")');
  assert.equal(a.runtime.run('get_back_face(card)'),null);
});

for(const fieldUpdateMode of ['lazy','ordered'])test(`${fieldUpdateMode} cards share set values and edits from either card`,async()=>{
  const t=await template('set field:\n\ttype: text\n\tname: title\n\tscript: input + "!"\ncard field:\n\ttype: text\n\tname: heading\n\tdefault: set.title + ": " + card.name\n');
  const set=new MseTemplateSet(t,{fieldUpdateMode,set:{title:'Collection'},cards:[{card:{name:'A'}},{card:{name:'B'}}]}).initialize();
  const [a,b]=set.contexts;
  assert.equal(a.cells.set,b.cells.set);
  assert.deepEqual(set.contexts.map(c=>text(c.field('card','heading'))),['Collection!: A','Collection!: B']);
  b.setField('set','title','Revised');
  assert.deepEqual(set.contexts.map(c=>text(c.field('card','heading'))),['Revised!: A','Revised!: B']);
  a.setField('set','title','Final');
  assert.deepEqual(set.contexts.map(c=>text(c.field('card','heading'))),['Final!: A','Final!: B']);
});
