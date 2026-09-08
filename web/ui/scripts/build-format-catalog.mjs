import { readFile, writeFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { createHash } from 'node:crypto';
import { PUBLIC_FORMATS } from '../src/lib/relay/formats.js';
const source = await readFile(new URL('../../../cards.json', import.meta.url), 'utf8');
const cards = {};
for (const card of JSON.parse(source)) {
  if (!card.name || !card.legalities) continue;
  const face = card.card_faces?.[0] || card;
  const oracle = card.oracle_text || (card.card_faces || []).map(f => f.oracle_text || '').join('\n');
  cards[card.name.toLowerCase()] = {
    name: card.name, legalities: Object.fromEntries(Object.keys(PUBLIC_FORMATS).map(f => [f, card.legalities[f] || 'not_legal'])),
    colors: card.color_identity || [], type: face.type_line || '',
    // Only rules relevant to deck construction, keeping the catalog small.
    rules: oracle.split('\n').filter(line => /deck can have|commander|partner|background|friends forever|Doctor's companion/i.test(line)).join('\n'),
  };
}
const metadata = JSON.parse(await readFile(new URL('../../../cards.json.scryfall-bulk-data.json', import.meta.url), 'utf8'));
const out = new URL('../src/lib/relay/format-catalog.generated.json', import.meta.url);
await writeFile(out, JSON.stringify({ sourceUpdatedAt: metadata.updated_at, sha256: createHash('sha256').update(source).digest('hex'), cards }));
console.log(`Wrote ${Object.keys(cards).length} cards to ${fileURLToPath(out)}`);
