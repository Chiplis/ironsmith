import * as en from './locales/en.js';
import * as es from './locales/es.js';

export const localeCatalogs = { en, es };
export const DEFAULT_LOCALE = 'en';
export const LOCALES = [{ id: 'en', label: 'English' }, { id: 'es', label: 'Español' }];
export const I18N_STORAGE_KEY = 'ironsmith.locale';
let activeLocale = DEFAULT_LOCALE;
try {
  const stored = globalThis.localStorage?.getItem(I18N_STORAGE_KEY);
  if (localeCatalogs[stored]) activeLocale = stored;
} catch { /* Storage may be unavailable in private/embedded browsers. */ }
export const getActiveLocale = () => activeLocale;
export function setActiveLocale(locale) {
  activeLocale = localeCatalogs[locale] ? locale : DEFAULT_LOCALE;
}

export function interpolate(template, params) {
  return String(template).replace(/\{([a-zA-Z0-9_]+)\}/g, (match, key) => (
    params?.[key] == null ? match : String(params[key])
  ));
}

// These captured fields are interface vocabulary, rather than player/card data.
// Keep this explicit: translating every captured word could rename a card or player.
const localizedParameters = {
  'Added {0} to {1}': ['1'],
  '{0} ability cannot be activated now: {1}': ['1'],
  '{0} {1} mana in pool': ['1'],
  '{0} {1} mana{2}': ['1', '2'],
  'Add card failed: {0}': ['0'],
  'Random game failed: {0}': ['0'],
};
function localizeParameters(source, params, catalog) {
  if (!params || !localizedParameters[source]) return params;
  const result = { ...params };
  for (const key of localizedParameters[source]) {
    const translated = catalog[result[key]];
    if (typeof translated === 'string') result[key] = translated;
  }
  return result;
}

const pluralRules = new Map();
function renderMessage(message, params, locale) {
  if (message && typeof message === 'object') {
    if (!pluralRules.has(locale)) pluralRules.set(locale, new Intl.PluralRules(locale));
    const category = pluralRules.get(locale).select(Number(params?.[message.count] ?? 0));
    return interpolate(message[category] ?? message.other, params);
  }
  return interpolate(message, params);
}

const sourceCatalogs = Object.fromEntries(Object.entries(localeCatalogs).map(([locale, catalog]) => {
  const sources = {};
  for (const [key, source] of Object.entries(en.messages)) sources[source] = catalog.messages[key] ?? source;
  return [locale, { ...sources, ...catalog.ui }];
}));
const escapeRegex = text => text.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
const templates = Object.entries(sourceCatalogs.en)
  .filter(([source]) => /\{\w+\}/.test(source) && /[A-Za-z]{3}/.test(source.replace(/\{\w+\}/g, '')))
  .map(([source]) => {
    const keys = [];
    const parts = source.split(/(\{\w+\})/g).map(part => {
      if (/^\{\w+\}$/.test(part)) { keys.push(part.slice(1, -1)); return '([\\s\\S]*?)'; }
      return escapeRegex(part);
    });
    return { source, keys, pattern: new RegExp(`^${parts.join('')}$`) };
  }).sort((a, b) => b.source.replace(/\{\w+\}/g, '').length - a.source.replace(/\{\w+\}/g, '').length);
const cache = new Map();

// Source-text lookup is for UI-owned labels and known engine status messages.
// Card/player names and protocol values must not be passed through this API.
export function translateUiText(value, params = null, locale = activeLocale) {
  if (typeof value !== 'string') return value;
  const catalog = sourceCatalogs[locale] || sourceCatalogs.en;
  if (params) return renderMessage(catalog[value] ?? value, localizeParameters(value, params, catalog), locale);
  if (locale === 'en') return value;
  if (Object.hasOwn(catalog, value)) return renderMessage(catalog[value], null, locale);
  const cacheKey = `${locale}:${value}`;
  if (cache.has(cacheKey)) return cache.get(cacheKey);
  let translated = value;
  for (const template of templates) {
    const match = template.pattern.exec(value);
    if (!match) continue;
    const captured = Object.fromEntries(template.keys.map((key, index) => [key, match[index + 1]]));
    translated = renderMessage(catalog[template.source] ?? template.source, localizeParameters(template.source, captured, catalog), locale);
    break;
  }
  if (cache.size >= 2000) cache.clear();
  cache.set(cacheKey, translated);
  return translated;
}
