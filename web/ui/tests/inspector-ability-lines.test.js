import test from 'node:test';
import assert from 'node:assert/strict';
import { interactiveRulesView, activatedAbilityLineIndices, abilityLinePositions } from '../src/lib/inspector-ability-lines.js';
import { groupManaAbilities } from '../src/lib/group-mana-abilities.js';
import { intrinsicManaColors, withoutIntrinsicManaLines } from '../src/lib/land-mana-choices.js';

test('a fetch land made a Forest never binds hidden green mana to its fetch line', () => {
  const fetchText = '{T}, Pay 1 life, Sacrifice this land: Search your library for a Mountain or Plains card, put it onto the battlefield, then shuffle.';
  const green = { kind: 'activate_mana_ability', ability_index: 1, ability_text: '{T}: Add {G}.' };
  const fetch = { kind: 'activate_ability', ability_index: 0, ability_text: fetchText };
  const visible = withoutIntrinsicManaLines([fetchText, green.ability_text], intrinsicManaColors('Land — Forest')).join('\n');
  assert.equal(visible, fetchText);
  for (const actions of [[green, fetch], [fetch, green], [green], []]) {
    for (const displayed of [visible, '{T}, Pagar 1 vida, sacrificar esta tierra: Busca en tu biblioteca una carta de Montaña o Llanura.']) {
      const view = interactiveRulesView(visible, displayed, actions);
      assert.deepEqual(view.actions.get(0) || [], actions.includes(fetch) ? [fetch] : []);
    }
  }
});

test('unmatched ability text cannot fall back to equal-count line ordering', () => {
  const text = '{T}: Draw a card.\n{2}: Gain 1 life.';
  const draw = { ability_text: '{T}: Draw a card.' };
  const green = { ability_text: '{T}: Add {G}.' };
  const view = interactiveRulesView(text, text, [draw, green]);
  assert.deepEqual([...view.actions], [[0, [draw]]]);
});

const canonical = 'Flying\n{T}: Add {G}.\n{4}, {T}: Draw a card.';
const draw = { index: 17, ability_index: 2, ability_text: '{4}, {T}: Draw a card.', action_ref: { id: 'draw' } };
const mana = { index: 8, ability_index: 1, ability_text: '{T}: Add {G}.' };
for (const [locale, translated] of Object.entries({
  es: 'Vuela\n{T}: Agrega {G}.\n{4}, {T}: Roba una carta.',
  fr: 'Vol\n{T} : Ajoutez {G}.\n{4}, {T} : Piochez une carte.',
  de: 'Fliegend\n{T}: Erzeuge {G}.\n{4}, {T}: Ziehe eine Karte.',
  ja: '飛行\n{T}：{G}を加える。\n{4}, {T}：カード１枚を引く。',
  zhs: '飞行\n{T}：加{G}。\n{4}, {T}：抓一张牌。',
})) {
  test(`${locale}: partial availability keeps the draw action on its translated ability`, () => {
    const view = interactiveRulesView(canonical, translated, [draw]);
    assert.equal(view.lines.join('\n'), translated);
    assert.deepEqual([...view.actions.keys()], [2]);
    assert.equal(view.actions.get(2)[0], draw);
    const all = interactiveRulesView(canonical, translated, [draw, mana]);
    assert.equal(all.actions.get(1)[0], mana);
    assert.equal(all.actions.get(2)[0], draw);
  });
}

test('extra translated reminder paragraphs do not shift actions', () => {
  const translated = 'Vuela\n(Recordatorio: texto adicional.)\n{T}: Agrega {G}.\n{4}, {T}: Roba una carta.';
  const view = interactiveRulesView(canonical, translated, [draw]);
  assert.deepEqual(activatedAbilityLineIndices(view.lines), [2, 3]);
  assert.equal(view.actions.get(3)[0], draw);
});

test('mana symbols distinguish reordered dual-land abilities', () => {
  const blue = { ability_text: '{T}: Add {U}.' };
  const view = interactiveRulesView('{T}: Add {G}.\n{T}: Add {U}.', '{T}: Agrega {U}.\n{T}: Agrega {G}.', [mana, blue]);
  assert.equal(view.actions.get(0)[0], blue);
  assert.equal(view.actions.get(1)[0], mana);
});

test('ambiguous translated structure retains usable canonical rules', () => {
  const view = interactiveRulesView(canonical, '{T}: Agrega {G}. {4}, {T}: Roba una carta.', [draw]);
  assert.equal(view.lines.join('\n'), canonical);
  assert.equal(view.actions.get(2)[0], draw);
});

test('missing text does not map a filtered action list by position', () => {
  const view = interactiveRulesView(canonical, canonical, [{ ability_index: 2 }]);
  assert.equal(view.actions.size, 0);
});

test('language switches preserve action identity and payment state', () => {
  const pending = { ...draw, payment_pending: true, mana_payment_available: false };
  const spanish = 'Vuela\n{T}: Agrega {G}.\n{4}, {T}: Roba una carta.';
  for (const text of [spanish, canonical, spanish]) {
    assert.equal(interactiveRulesView(canonical, text, [pending]).actions.get(2)[0], pending);
  }
});

test('keyword lines pair with their translations in order so printed paragraphs can be replaced', () => {
  const canonical = 'Protection from Humans\nPay 1 life, Sacrifice another creature: Draw a card.\n{B}{B}, Discard a card: Proliferate.';
  const translated = 'Protección contra Humanos.\nPagar 1 vida, sacrificar otra criatura: Roba una carta.\n{B}{B}, descartar una carta: Prolifera.';
  const view = interactiveRulesView(canonical, translated, []);
  assert.deepEqual(view.sourceLines, [['Protection from Humans'], ['Pay 1 life, Sacrifice another creature: Draw a card.'], ['{B}{B}, Discard a card: Proliferate.']]);
  // A translation that splits reminder text into its own paragraph is ambiguous
  // for the static lines and leaves them unpaired rather than mismatched.
  const split = interactiveRulesView('Flying\nTrample', 'Vuela.\n(Reminder.)\nArrolla.', []);
  assert.deepEqual(split.sourceLines, [[], [], []]);
});

test('empty live abilities suppress printed rules after ability removal', async()=>{
  const {inspectorRulesLines}=await import('../src/lib/inspector-ability-lines.js');
  const printed='Each land is a Forest in addition to its other land types.';
  assert.deepEqual(inspectorRulesLines([],[],printed),[]);
  assert.deepEqual(inspectorRulesLines([], [printed],printed),[]);
  assert.deepEqual(inspectorRulesLines(null,[],printed),[]);
  assert.deepEqual(inspectorRulesLines(null,null,printed),[printed]);
  assert.deepEqual(inspectorRulesLines([printed],[],printed),[printed]);
  assert.deepEqual(inspectorRulesLines(['Mana ability: {T}: Add {R}.'],[],printed),['{T}: Add {R}.']);
});

test('Equip keyword rows stay actionable alongside static equipment text and in Spanish', () => {
  const canonical = 'Equipped creature gets +1/+1.\nEquip {1}';
  const action = { kind: 'activate_ability', index: 7, label: 'Activate Test Equipment: Equip {1}' };
  assert.deepEqual(activatedAbilityLineIndices(canonical.split('\n')), [1]);
  for (const displayed of [canonical, 'La criatura equipada obtiene +1/+1.\nEquipar {1}']) {
    const view = interactiveRulesView(canonical, displayed, [action]);
    assert.equal(view.lines.join('\n'), displayed);
    assert.equal(view.actions.get(1)[0], action);
    assert.equal(view.actions.has(0), false);
  }
});

test('a discounted Equip action cannot highlight the static Equipped creature line', () => {
  const text = 'Equipped creature gets +1/+1.\nEquip {1}';
  const action = { kind: 'activate_ability', label: 'Activate Equipment: Equip {0}' };
  const view = interactiveRulesView(text, text, [action]);
  assert.equal(view.actions.has(0), false);
  assert.equal(view.actions.get(1)[0], action);
});

test('intrinsic starting counters are hidden without hiding real counter abilities', async()=>{
  const {inspectorRulesLines}=await import('../src/lib/inspector-ability-lines.js');
  const {cardFrameSnapshot}=await import('../src/lib/card-frame-snapshot.mjs');
  const real='This permanent enters with two additional loyalty counters on it.';
  const ability='+1: Draw a card.';
  const lines=['Enters with loyalty counters equal to its printed loyalty number',ability,real,
    'Enters with defense counters equal to its printed defense number'];
  assert.deepEqual(inspectorRulesLines(lines),[ability,real]);
  const action={ability_index:1,ability_text:ability};
  const view=interactiveRulesView(lines.join('\n'),lines.join('\n'),[action]);
  assert.deepEqual(view.lines,[ability,real]);
  assert.equal(view.actions.get(0)[0],action);
  assert.equal(JSON.parse(cardFrameSnapshot({oracle_text:lines.join('\n')})).oracle_text,[ability,real].join('\n'));
});

test('custom activation declarations bind their expanded action to the printed row',()=>{
 const definitions=[{keyword:'improve',template:'{Cost}',reminder:'{Cost}: Put a +1/+1 counter on this permanent. This ability costs {1} more to activate for each +1/+1 counter on this permanent. Activate only as a sorcery.'}];
 const text='Improve {B}\nWhenever you put one or more +1/+1 counters on this creature, reveal the top card of your library and put it into your hand. You lose life equal to its mana value.';
 const action={kind:'activate_ability',ability_text:'{B}: Put a +1/+1 counter on this permanent. Activate only as a sorcery.'};
 const view=interactiveRulesView(text,text,[action],definitions);
 assert.deepEqual(activatedAbilityLineIndices(view.lines,definitions),[0]);
 assert.deepEqual([...view.actions],[[0,[action]]]);
});
test('Equip X maps expanded activation text to its keyword row',()=>{
 const text='Equipped creature gets +1/+1.\nEquip {X}';
 const action={kind:'activate_ability',ability_text:'{X}: Attach this Equipment to target creature you control. Activate only as a sorcery.'};
 assert.deepEqual([...interactiveRulesView(text,text,[action]).actions],[[1,[action]]]);
});

test('Advance finds its activation after casting and entry clauses in the imported reminder',async()=>{
 const {readFileSync}=await import('node:fs');
 const set=JSON.parse(readFileSync(new URL('../public/custom-sets/msem-KXP.json',import.meta.url)));
 const draft=set.cards.find(c=>c.draft.faces[0].name==='Shearing Vampire').draft;
 const text=draft.faces[0].oracleText;
 const action={kind:'activate_ability',ability_text:'{4}{B}: Have this permanent perform the advance action. Activate only as a sorcery.'};
 assert.deepEqual(activatedAbilityLineIndices(text.split('\n'),draft.customMechanics),[0]);
 assert.deepEqual([...interactiveRulesView(text,text,[action],draft.customMechanics).actions],[[0,[action]]]);
});
test('compiled Improve identity is never a second activation or an unavailable row',async()=>{
 const {readFileSync}=await import('node:fs');
 const set=JSON.parse(readFileSync(new URL('../public/custom-sets/msem-MSY.json',import.meta.url)));
 const definitions=set.cards.find(c=>c.draft.faces[0].name==='Herald of Tedium').draft.customMechanics;
 const action={kind:'activate_ability',ability_text:'{B}: Put a +1/+1 counter on this creature. Activate only as a sorcery.'};
 const text=`Improve.\n${action.ability_text}\nThis ability costs {1} more to activate for each +1/+1 counter on this permanent.\nWhenever you put one or more +1/+1 counters on this creature, reveal the top card of your library and put it into your hand.`;
 assert.deepEqual(activatedAbilityLineIndices(text.split('\n'),definitions),[1]);
 assert.deepEqual([...interactiveRulesView(text,text,[action],definitions).actions],[[1,[action]]]);
});

test('a bare Station declaration is an actionable row, not only a tooltip', () => {
  const text = 'Station\n20+ | {T}: Add X mana of any one color, where X is the number of charge counters on The Eternity Elevator.';
  assert.deepEqual(activatedAbilityLineIndices(text.split('\n')), [0, 1]);
  const station = { kind: 'activate_ability', label: 'Activate The Eternity Elevator: Station' };
  assert.deepEqual(interactiveRulesView(text, text, [station]).actions.get(0), [station]);
});

const elevator = 'Station\n20+ | {T}: Add X mana of any one color, where X is the number of charge counters on The Eternity Elevator.';
const elevatorAbilities = [
  { ability_index: 0, kind: 'activated', text: 'Station' },
  { ability_index: 1, kind: 'activated', text: '20+ | {T}: Add X mana of any one color, where X is the number of charge counters on The Eternity Elevator.' },
];

test('engine ability indices bind actions exactly, whatever their wording', () => {
  const station = { kind: 'activate_ability', ability_index: 0, label: 'Activate: put counters' };
  const mana = { kind: 'activate_mana_ability', ability_index: 1, label: 'Add mana' };
  const view = interactiveRulesView(elevator, elevator, [mana, station], [], elevatorAbilities);
  assert.deepEqual(view.actions.get(0), [station]);
  assert.deepEqual(view.actions.get(1), [mana]);
  assert.deepEqual([...view.activatedLines], [0, 1]);
});

test('a translation with the same paragraphs keeps exact bindings without recognizing its wording', () => {
  const station = { kind: 'activate_ability', ability_index: 0, label: 'Station' };
  const spanish = 'Estación\n20+ | {T}: Agrega X maná de un color cualquiera.';
  const view = groupManaAbilities(interactiveRulesView(elevator, spanish, [station], [], elevatorAbilities), 'es');
  assert.equal(view.lines[0], 'Estación');
  assert.deepEqual(view.actions.get(0), [station]);
  assert.deepEqual([...view.activatedLines], [0, 1]);
});

test('modal abilities bind to their first line and identical lines are claimed in order', () => {
  const lines = ['{T}: Add {C}.', '{T}: Add {C}.', '{1}: Choose one —', '• Draw a card.', '• Gain 2 life.'];
  const { byAbility, activated } = abilityLinePositions(lines, [
    { ability_index: 0, kind: 'activated', text: '{T}: Add {C}.' },
    { ability_index: 1, kind: 'activated', text: '{T}: Add {C}.' },
    { ability_index: 2, kind: 'activated', text: '{1}: Choose one —\n• Draw a card.\n• Gain 2 life.' },
  ]);
  assert.deepEqual([...byAbility], [[0, 0], [1, 1], [2, 2]]);
  assert.deepEqual([...activated], [0, 1, 2]);
});

test('non-ability payment sources never bind by ability index', () => {
  const convoke = { kind: 'plan_payment_source', payment_kind: 'convoke', ability_index: 0, label: 'Tap Llanowar Elves' };
  const view = interactiveRulesView(elevator, elevator, [convoke], [], elevatorAbilities);
  assert.equal(view.actions.get(0), undefined);
});
