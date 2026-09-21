import { manaAssets, manaGlyphs } from './mana-assets.generated.js';

const base = import.meta.env?.BASE_URL || '/';
export function manaSymbolUrl(symbol) {
  const asset = manaAssets[String(symbol || '').trim().toUpperCase()];
  return asset ? `${base}mana/symbols/${asset}.svg` : null;
}

export function counterSymbolUrl(kind) {
  const key = String(kind || '').trim().toLowerCase().replaceAll('_', ' ').replace(/\s+/g, '-');
  const alias = {
    'plus-one-plus-one': 'counter-plus', '+1/+1': 'counter-plus',
    'minus-one-minus-one': 'counter-minus', '-1/-1': 'counter-minus',
    energy: 'e', poison: 'ability-toxic', finality: 'counter-skull',
  }[key];
  const asset = alias || [`counter-${key}`, `ability-${key.replaceAll('-', '')}`, `ability-${key}`].find(name => manaGlyphs.includes(name));
  return asset ? `${base}mana/svg/${asset}.svg` : null;
}
