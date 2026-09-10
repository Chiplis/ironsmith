import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';

// The build script runs on import, so exercise its text rules by evaluating
// the helper source it exports without executing the pipeline.
async function loadHelpers() {
  const source = await readFile(new URL('../scripts/build-scryfall-i18n.mjs', import.meta.url), 'utf8');
  const pick = name => {
    const start = source.indexOf(`function ${name}(`);
    const exportStart = source.lastIndexOf('export ', start);
    const from = exportStart >= 0 && source.slice(exportStart, start).trim() === 'export' ? exportStart : start;
    let depth = 0, i = source.indexOf('{', start);
    for (; i < source.length; i++) { if (source[i] === '{') depth++; else if (source[i] === '}' && --depth === 0) break; }
    return source.slice(from, i + 1).replace(/^export\s+/, '');
  };
  const code = ['normalizeText', 'wordTokens', 'looksUntranslated', 'hasTranslatableWords', 'firstFaceValue', 'localizedPrintedText'].map(pick).join('\n');
  return import(`data:text/javascript,${encodeURIComponent(`${code}\nexport {hasTranslatableWords, localizedPrintedText, wordTokens};`)}`);
}

test('a bare mana letter is not a translation of basic-land reminder text', async () => {
  const {hasTranslatableWords, localizedPrintedText, wordTokens} = await loadHelpers();
  const english = {textNorm: '({T}: Add {B}.)', tokens: wordTokens('({T}: Add {B}.)')};
  assert.equal(hasTranslatableWords('B'), false);
  assert.equal(hasTranslatableWords('{B}'), false);
  assert.equal(localizedPrintedText(english, {printed_text: 'B'}), '');
  assert.equal(localizedPrintedText(english, {printed_text: '({T}: Agrega {B}.)'}), '({T}: Agrega {B}.)');
  assert.equal(localizedPrintedText(english, {printed_text: '({T}: Add {B}.)'}), '');
});

test('face routes never shadow a card that owns the route by its full name', async () => {
  const source = await readFile(new URL('../scripts/build-scryfall-i18n.mjs', import.meta.url), 'utf8');
  const start = source.indexOf('export function nameRouteEntries(');
  let depth = 0, i = source.indexOf('{', start);
  for (; i < source.length; i++) { if (source[i] === '{') depth++; else if (source[i] === '}' && --depth === 0) break; }
  const {nameRouteEntries} = await import(`data:text/javascript,${encodeURIComponent(source.slice(start, i + 1))}`);
  const bolt = {route: 'lightning-bolt', englishName: 'Lightning Bolt'};
  const emeritus = {route: 'emeritus-of-conflict-lightning-bolt', englishName: 'Emeritus of Conflict // Lightning Bolt'};
  const faces = payload => payload === emeritus ? ['emeritus-of-conflict', 'lightning-bolt'] : [];
  for (const order of [[bolt, emeritus], [emeritus, bolt]]) {
    const routes = new Map(nameRouteEntries(order, faces));
    assert.equal(routes.get('lightning-bolt'), bolt);
    assert.equal(routes.get('emeritus-of-conflict'), emeritus);
    assert.equal(routes.get('emeritus-of-conflict-lightning-bolt'), emeritus);
  }
});

test('empty printed names and type lines are filled from the newest sibling printing', async () => {
  const source = await readFile(new URL('../scripts/build-scryfall-i18n.mjs', import.meta.url), 'utf8');
  const pick = name => {
    const start = source.indexOf(`export function ${name}(`);
    let depth = 0, i = source.indexOf('{', start);
    for (; i < source.length; i++) { if (source[i] === '{') depth++; else if (source[i] === '}' && --depth === 0) break; }
    return source.slice(start, i + 1);
  };
  const {rememberFieldFills, backfillPayloadFields} = await import(`data:text/javascript,${encodeURIComponent(pick('rememberFieldFills') + pick('backfillPayloadFields'))}`);
  const fills = new Map();
  rememberFieldFills(fills, 'o1', {name: 'Relámpago', typeLine: 'Instantáneo'}, '2010-07-16');
  rememberFieldFills(fills, 'o1', {name: 'Relámpago', typeLine: ''}, '2022-10-14');
  rememberFieldFills(fills, 'o1', {name: '', typeLine: 'Instantáneo (viejo)'}, '2009-07-17');
  const payload = backfillPayloadFields({oracleId: 'o1', name: 'Relámpago', typeLine: '', oracleText: 'El Relámpago hace 3 puntos de daño a cualquier objetivo.'}, fills.get('o1'));
  assert.equal(payload.typeLine, 'Instantáneo', 'newest printing with a type line wins');
  assert.equal(payload.oracleText, 'El Relámpago hace 3 puntos de daño a cualquier objetivo.', 'rules text stays with the chosen printing');
  assert.equal(backfillPayloadFields({name: 'x', typeLine: 'y'}, fills.get('o1')).typeLine, 'y', 'present fields are kept');
  assert.equal(backfillPayloadFields({name: '', typeLine: ''}, undefined).name, '');
});
