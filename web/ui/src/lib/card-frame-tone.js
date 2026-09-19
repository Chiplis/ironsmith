import { normalizeArtColors } from './card-art-colors.js';

// The synthetic frame's material follows the card's colors, the way a printed
// frame does: one color paints the whole frame, two colors split the frame
// between them under gold bars, three or more are plain gold, and a colorless
// card falls back to what it is (a land, an artifact, or neither).
const PALETTES = {
  white:     {edge: '#b5ad91', edgeLight: '#eee7c9', bar: '#d8d0ae', barLight: '#f3edd7', paper: '#ece7d4', paperDeep: '#d4ccb0', accent: '#a99a67'},
  blue:      {edge: '#467995', edgeLight: '#acd8e8', bar: '#72a5bd', barLight: '#c4e4ee', paper: '#d6e4e7', paperDeep: '#a9c7d0', accent: '#397995'},
  black:     {edge: '#4b4648', edgeLight: '#9d9297', bar: '#716b6d', barLight: '#aaa1a4', paper: '#cec8c7', paperDeep: '#aba1a3', accent: '#57474f'},
  red:       {edge: '#8b473b', edgeLight: '#e2a08a', bar: '#ae6654', barLight: '#e5b19d', paper: '#e6d1c7', paperDeep: '#caa395', accent: '#9b493b'},
  green:     {edge: '#3f7054', edgeLight: '#9bc5a7', bar: '#659275', barLight: '#c1dcc7', paper: '#d4e0d3', paperDeep: '#aac2ae', accent: '#39724d'},
  gold:      {edge: '#987c41', edgeLight: '#e5d18d', bar: '#b89b55', barLight: '#eadca7', paper: '#e0dac2', paperDeep: '#c4b98c', accent: '#997b35'},
  land:      {edge: '#725f48', edgeLight: '#c6ad85', bar: '#958064', barLight: '#d3c2a3', paper: '#ddd3bd', paperDeep: '#bca98b', accent: '#776043'},
  artifact:  {edge: '#96999d', edgeLight: '#d8dde1', bar: '#aeb2b5', barLight: '#e2e5e6', paper: '#d7d8d4', paperDeep: '#b9bcb8', accent: '#788087'},
  colorless: {edge: '#a8a396', edgeLight: '#e2ddd1', bar: '#c4bfb2', barLight: '#e9e5db', paper: '#e3dfd5', paperDeep: '#c7c2b5', accent: '#8c8676'},
};
const COLOR_TONES = {W: 'white', U: 'blue', B: 'black', R: 'red', G: 'green'};

// A card's live colors when the object states them (the engine's signature or
// a colors field, both of which include effects such as devoid), or null when
// the object carries no color information at all: an empty list is a real
// answer (colorless) and must not fall through to a weaker source.
export function explicitCardColors(card) {
  if (!card || typeof card !== 'object') return null;
  const signature = String(card.characteristic_signature || '').match(/^colors:([^\n]*)/m);
  if (signature) return normalizeArtColors(signature[1]);
  if (card.colors != null) return normalizeArtColors(card.colors);
  return null;
}

export function manaCostColors(manaCost) {
  const symbols = [...String(manaCost || '').matchAll(/\{([^}]+)\}/g)];
  return normalizeArtColors(symbols.flatMap(match => match[1].split('/')));
}

// Live objects first (the engine knows about color-changing effects), then the
// displayed printing (Scryfall states a face's colors, including color
// indicators and tokens), then the mana cost as the last resort.
export function cardFrameColors({cards = [], printing = null, manaCost = ''} = {}) {
  for (const card of cards) {
    const colors = explicitCardColors(card);
    if (colors) return colors;
  }
  if (Array.isArray(printing?.colors)) return normalizeArtColors(printing.colors);
  return manaCostColors(manaCost);
}

export function frameToneForColors(colors, typeLine = '') {
  const normalized = normalizeArtColors(colors);
  if (normalized.length === 1) return COLOR_TONES[normalized[0]];
  if (normalized.length > 1) return 'gold';
  const type = String(typeLine || '').toLowerCase();
  if (/\bland\b/.test(type)) return 'land';
  if (/\bartifact\b/.test(type)) return 'artifact';
  return 'colorless';
}

function paletteStyle(palette) {
  return {
    '--card-frame-edge': palette.edge,
    '--card-frame-edge-light': palette.edgeLight,
    '--card-frame-bar': palette.bar,
    '--card-frame-bar-light': palette.barLight,
    '--card-frame-paper': palette.paper,
    '--card-frame-paper-deep': palette.paperDeep,
    '--card-frame-accent': palette.accent,
  };
}

// The stage's inline style: the palette as the frame variables, and for a
// two-color card the split edge fill in which each half keeps its own color.
export function cardFrameToneStyle(tone, colors = []) {
  const style = paletteStyle(PALETTES[tone] || PALETTES.colorless);
  const normalized = normalizeArtColors(colors);
  if (tone === 'gold' && normalized.length === 2) {
    const [first, second] = normalized.map(color => PALETTES[COLOR_TONES[color]]);
    style['--card-frame-edge-fill'] = `linear-gradient(90deg, ${first.edge} 0%, ${first.edge} 36%, ${second.edge} 64%, ${second.edge} 100%)`;
    style['--card-frame-edge-light'] = `color-mix(in srgb, ${first.edgeLight}, ${second.edgeLight})`;
    style['--card-frame-accent'] = PALETTES.gold.accent;
  }
  return style;
}

export function cardFrameTone(input) {
  const colors = cardFrameColors(input);
  const tone = frameToneForColors(colors, input?.typeLine);
  return {tone, colors, style: cardFrameToneStyle(tone, colors)};
}
