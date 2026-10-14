// Snapshots the cards that may lead a Pauper EDH deck. Scryfall's bulk
// legalities only cover the 99 (commons); commander eligibility depends on a
// card's printings at uncommon, which only its `is:paupercommander` search
// knows. build-format-catalog.mjs reads the snapshot this writes.
import { writeFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';

const out = new URL('./data/pdh-commanders.json', import.meta.url);
const names = new Set();
let url = `https://api.scryfall.com/cards/search?${new URLSearchParams({ q: 'is:paupercommander', unique: 'cards', order: 'name' })}`;
while (url) {
  let response;
  for (let attempt = 0; attempt < 5; attempt++) {
    response = await fetch(url, { headers: { 'user-agent': 'IronSmith format catalog', accept: 'application/json' } });
    if (response.status !== 429) break;
    await new Promise(resolve => setTimeout(resolve, 1000 * (Number(response.headers.get('retry-after')) || 15 * (attempt + 1))));
  }
  if (!response.ok) throw new Error(`Scryfall search failed (${response.status})`);
  const page = await response.json();
  for (const card of page.data || []) names.add(card.name);
  url = page.has_more ? page.next_page : '';
  // Scryfall rate-limits its search endpoint well below its general API.
  if (url) await new Promise(resolve => setTimeout(resolve, 1000));
}
await writeFile(out, `${JSON.stringify({ fetchedAt: new Date().toISOString(), query: 'is:paupercommander', names: [...names].sort() }, null, 1)}\n`);
console.log(`Wrote ${names.size} Pauper EDH commanders to ${fileURLToPath(out)}`);
