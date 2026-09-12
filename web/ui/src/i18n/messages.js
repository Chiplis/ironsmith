// Compatibility API: the locale registry is the only list of supported languages.
import { localeCatalogs } from './catalog.js';
export const messages = Object.fromEntries(
  Object.entries(localeCatalogs).map(([locale, catalog]) => [locale, catalog.messages])
);
