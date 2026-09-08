import test from 'node:test';
import assert from 'node:assert/strict';
import { interactiveRulesView, activatedAbilityLineIndices } from '../src/lib/inspector-ability-lines.js';

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
