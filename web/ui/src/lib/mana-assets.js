import { manaAssets, manaGlyphs } from './mana-assets.generated.js';

const base = import.meta.env?.BASE_URL || '/';

const POWER_TOUGHNESS_COUNTER_ALIASES = {
  'plus one plus one': '+1/+1',
  'minus one minus one': '-1/-1',
  'plus one plus zero': '+1/+0',
  'plus zero plus one': '+0/+1',
  'plus one plus two': '+1/+2',
  'plus two plus two': '+2/+2',
  'minus zero minus one': '-0/-1',
  'minus zero minus two': '-0/-2',
  'minus two minus one': '-2/-1',
  'minus two minus two': '-2/-2',
};

/** Returns the canonical visible label for any numeric P/T counter kind. */
export function counterDisplayLabel(kind) {
  const raw = String(kind || '').trim();
  const normalized = raw.toLowerCase().replaceAll('_', ' ').replace(/\s+/g, ' ');
  if (POWER_TOUGHNESS_COUNTER_ALIASES[normalized]) {
    return POWER_TOUGHNESS_COUNTER_ALIASES[normalized];
  }
  return /^[+-]\d+\/[+-]\d+$/.test(raw) ? raw : null;
}

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
