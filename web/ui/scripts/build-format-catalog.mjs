import { readFile, writeFile } from 'node:fs/promises';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { createHash } from 'node:crypto';
import { PUBLIC_FORMATS } from '../src/lib/relay/formats.js';
// rebuild-wasm.sh passes its --cards-file; the repo's cards.json otherwise.
const cardsFile = process.env.IRONSMITH_CARDS_FILE
  ? pathToFileURL(process.env.IRONSMITH_CARDS_FILE)
  : new URL('../../../cards.json', import.meta.url);
const source = await readFile(cardsFile, 'utf8');
// Cards that may lead a Pauper EDH deck (refresh with fetch-pdh-commanders.mjs).
const pdhCommanders = new Set(JSON.parse(await readFile(new URL('./data/pdh-commanders.json', import.meta.url), 'utf8')).names);
const cards = {};
for (const card of JSON.parse(source)) {
  if (!card.name || !card.legalities) continue;
  const face = card.card_faces?.[0] || card;
  const oracle = card.oracle_text || (card.card_faces || []).map(f => f.oracle_text || '').join('\n');
  cards[card.name.toLowerCase()] = {
    name: card.name, legalities: Object.fromEntries(Object.keys(PUBLIC_FORMATS).map(f => [f, card.legalities[f] || 'not_legal'])),
    colors: card.color_identity || [], type: face.type_line || '',
    ...(pdhCommanders.has(card.name) ? { pdhCommander: true } : {}),
    // Previewed on Scryfall but legal nowhere yet (scripts/download_scryfall_cards.py).
    ...(card.ironsmith_unreleased === true ? { unreleased: true } : {}),
    // Only rules relevant to deck construction, keeping the catalog small.
    rules: oracle.split('\n').filter(line => /deck can have|as you create your deck|commander|partner|background|friends forever|Doctor's companion/i.test(line)).join('\n'),
  };
}
const metadata = JSON.parse(await readFile(new URL(`${cardsFile.href}.scryfall-bulk-data.json`), 'utf8'));
const out = new URL('../src/lib/relay/format-catalog.generated.json', import.meta.url);
await writeFile(out, JSON.stringify({ sourceUpdatedAt: metadata.updated_at, sha256: createHash('sha256').update(source).digest('hex'), cards }));
console.log(`Wrote ${Object.keys(cards).length} cards to ${fileURLToPath(out)}`);
