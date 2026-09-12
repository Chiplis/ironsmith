import test from 'node:test';
import assert from 'node:assert/strict';
import { localeCatalogs, translateUiText, setActiveLocale } from '../src/i18n/catalog.js';
import { MTG_KEYWORD_RULES } from '../src/lib/mtg-keyword-rules.generated.js';
import { getMtgKeywordRule, splitTextWithMtgKeywordRules } from '../src/lib/mtg-keywords.js';
import { auditUiTranslations } from '../scripts/check-ui-i18n.mjs';
const placeholders = text => [...new Set([...text.matchAll(/\{(\w+)\}/g)].map(match => match[1]))].sort();
test('every locale covers all interface keys and preserves template data', () => {
  for (const [locale, catalog] of Object.entries(localeCatalogs)) {
    for (const group of ['messages', 'ui']) {
      assert.deepEqual(Object.keys(catalog[group]).sort(), Object.keys(localeCatalogs.en[group]).sort(), `${locale}.${group} keys`);
      for (const [key, value] of Object.entries(catalog[group])) {
        const english = localeCatalogs.en[group][key];
        const variants = typeof value === 'string' ? [value] : [value.one, value.other];
        const expected = placeholders(english).filter(key => !value.omit?.includes(key));
        for (const variant of variants) {
          assert.ok(typeof variant === 'string' && variant.trim(), `${locale}: ${key} is empty`);
          assert.deepEqual(placeholders(variant), expected, `${locale}: ${key} placeholders`);
        }
      }
    }
    if (locale !== 'en') for (const rule of MTG_KEYWORD_RULES) {
      assert.ok(catalog.rules?.[rule.id]?.title, `${locale}: ${rule.id} title`);
      assert.ok(catalog.rules?.[rule.id]?.summary, `${locale}: ${rule.id} summary`);
    }
  }
});
test('visible literals and accessibility copy use the central catalog', () => {
  assert.deepEqual(auditUiTranslations(), []);
});
test('translations update with the locale and preserve interpolated names verbatim', () => {
  setActiveLocale('es');
  assert.equal(translateUiText('Battlefield'), 'Campo de batalla');
  assert.equal(translateUiText('{0} joined as player {1}', {0: 'Main', 1: 2}), 'Main se unió como jugador 2');
  assert.equal(translateUiText('Added Main to hand'), 'Se añadió Main a mano');
  assert.equal(translateUiText('Main joined as player 2'), 'Main se unió como jugador 2');
  assert.equal(translateUiText('No such application message'), 'No such application message');
  assert.equal(translateUiText(7), 7);
  setActiveLocale('en');
  assert.equal(translateUiText('Battlefield'), 'Battlefield');
});
test('Spanish plurals do not reuse English suffixes', () => {
  assert.equal(translateUiText('{0} action{1}', {0:2,1:'s'}, 'es'), '2 acciones');
  assert.equal(translateUiText('2 actions', null, 'es'), '2 acciones');
  assert.equal(translateUiText('1 counter', null, 'es'), '1 contador');
  assert.equal(translateUiText('2 counters', null, 'es'), '2 contadores');
  assert.equal(translateUiText('Stack (1 item)', null, 'es'), 'Pila (1 objeto)');
});

test('localized keyword help recognizes Spanish without matching inside longer words', () => {
  assert.equal(getMtgKeywordRule('Vuela', 'es')?.title, 'Flying');
  assert.equal(getMtgKeywordRule('Vuela', 'en'), null);
  const segments = splitTextWithMtgKeywordRules('Vuela. Vigilancia. sobrevuela.', 'es');
  assert.deepEqual(segments.filter(segment => segment.type === 'keyword').map(segment => segment.rule.title), ['Flying', 'Vigilance']);
});
