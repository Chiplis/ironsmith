import catalog from './default-card-frames.generated.json' with { type: 'json' };
import nonstandardNames from './nonstandard-card-layouts.generated.json' with { type: 'json' };
import { cardFrameColors, manaCostColors } from './card-frame-tone.js';
import { normalizeArtColors } from './card-art-colors.js';
import { sourceMaskLayoutGap } from './card-frame-layout.js';

const excludedNames = new Set(nonstandardNames);
const base = import.meta.env?.BASE_URL || '/';
// A URL inside a CSS variable resolves against the stylesheet that uses it.
// Production CSS lives in assets/, so resolve relative Vite bases against the
// document first, preserving installations served from a subdirectory.
const frameBase = typeof document === 'undefined' ? base : new URL(base, document.baseURI).href;
const frames = Object.fromEntries(Object.entries(catalog).map(([key, entry]) => [key, {
  key: `default:${key}`, defaultFrame: key, registration: null, printing: null,
  flavorText: '', artReady: true, typography: entry.typography,
  style: { ...entry.style, '--source-frame-image': `url("${frameBase}card-frames/default/${key}.webp")` },
}]));

export function fallbackCardFrameColors({ cards = [], printing = null, manaCost = '', typeLine = '' } = {}) {
  const sources = [...cards.filter(Boolean), printing].filter(Boolean);
  // A land also takes the intrinsic mana of its current basic land types
  // (Yavimaya makes every land a Forest), which identity does not reflect.
  const intrinsic = /\bLand\b/i.test(typeLine) ? sources.flatMap(card => card.intrinsic_mana || []) : [];
  // An explicit empty identity is authoritative too. Search every source
  // before deriving anything from the live object's current characteristics.
  for (const card of sources) {
    const identity = card.color_identity ?? card.colorIdentity;
    if (identity != null) return normalizeArtColors([...normalizeArtColors(identity), ...intrinsic]);
  }
  // Engine snapshots may omit identity. Use printed costs/rules and basic
  // land types, including other faces when available, until metadata arrives.
  const faces = [{ type_line: typeLine }, ...sources.flatMap(card => [card, ...(card.card_faces || [])])];
  const colors = manaCostColors(manaCost);
  for (const card of faces) {
    const rules = card.oracle_text ?? card.compiled_text ?? card.ability_text ?? card.effect_text ?? '';
    let text = Array.isArray(rules) ? rules.join('\n') : String(rules);
    // Reminder text does not contribute to identity (e.g. extort).
    while (/\([^()]*\)/.test(text)) text = text.replace(/\([^()]*\)/g, '');
    colors.push(...manaCostColors(`${card.mana_cost || ''} ${text}`));
    colors.push(...normalizeArtColors(card.color_indicator || []));
    const type = card.type_line || typeLine;
    if (/\bLand\b/i.test(type)) {
      for (const [subtype, color] of Object.entries({ Plains: 'W', Island: 'U', Swamp: 'B', Mountain: 'R', Forest: 'G' })) {
        if (new RegExp(`\\b${subtype}\\b`, 'i').test(type)) colors.push(color);
      }
    }
  }
  return normalizeArtColors([...colors, ...intrinsic, ...cardFrameColors({ cards, printing, manaCost })]);
}

export function isSagaCardFrame({ cards = [], printing = null, typeLine = '' } = {}) {
  return /\bSaga\b/i.test(typeLine) || [...cards, printing].some(card =>
    card && (card.layout === 'saga' || /\bSaga\b/i.test(card.type_line || ''))
  );
}

export function defaultCardFrameKey({ cards = [], printing = null, manaCost = '', typeLine = '' } = {}) {
  const sources = [...cards.filter(Boolean), printing].filter(Boolean);
  const type = typeLine || sources.find(card => card.type_line)?.type_line || '';
  // Sagas use the ordinary identity frame with live chapter text and a fitted
  // art crop; source-mask eligibility still guards the original scan separately.
  const saga = isSagaCardFrame({ cards, printing, typeLine: type });
  if (!type || (!saga && (sources.some(card =>
    (card.layout && card.layout !== 'normal')
    || excludedNames.has(String(card.name || '').toLowerCase())
    || sourceMaskLayoutGap({ ...card, type_line: card.type_line || type, layout: card.layout || 'normal' })
  ) || sourceMaskLayoutGap({ type_line: type, layout: 'normal' })
    || /\b(Class|Case|Room)\b/i.test(type)))) return null;
  const colors = fallbackCardFrameColors({ cards, printing, manaCost, typeLine: type });
  if (colors.length) return colors.join('');
  if (/\bLand\b/i.test(type)) return 'land';
  return /\bArtifact\b/i.test(type) ? 'C' : 'colorless';
}

// Synchronous selection: neither metadata fetches nor pixel masking are on the
// rendering path for standard layouts. The current card still supplies its art.
export function defaultCardFrame(input) {
  return frames[defaultCardFrameKey(input)] || null;
}
