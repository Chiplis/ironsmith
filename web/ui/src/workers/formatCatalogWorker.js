import { indexFormatCatalog } from '../lib/relay/format-catalog-index.js';
self.onmessage = async () => {
  try {
    const response = await fetch(new URL('../lib/relay/format-catalog.generated.json', import.meta.url));
    if (!response.ok) throw new Error('Card legality catalog unavailable. Try again before joining.');
    const data = await response.json();
    if (!data.cards || !data.sourceUpdatedAt) throw new Error('Invalid card legality catalog');
    self.postMessage({ data, aliases: indexFormatCatalog(data.cards) });
  } catch (error) { self.postMessage({ error: error.message }); }
};
