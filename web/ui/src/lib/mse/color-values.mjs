import { MseColor, MseKeywordParameter, force, conversion } from './script-values.mjs';

// MSE overrides silver/gold and uses wx colour values (green is bright green).
// Unknown names remain explicit conversion failures, never substituted black.
const names = {
  black: [0, 0, 0], white: [255, 255, 255], red: [255, 0, 0],
  green: [0, 255, 0], blue: [0, 0, 255], yellow: [255, 255, 0],
  cyan: [0, 255, 255], magenta: [255, 0, 255],
  grey: [128, 128, 128], gray: [128, 128, 128],
  silver: [200, 200, 200], gold: [200, 180, 0], transparent: [0, 0, 0, 0],
};
export function color(value) {
  value = force(value);
  if (value instanceof MseKeywordParameter) value = value.value;
  if (value instanceof MseColor) return value;
  if (value === null) return new MseColor(0, 0, 0, 0);
  if (typeof value !== 'string') return conversion(value, 'color');
  // Native scanf accepts a complete channel prefix without requiring the
  // closing delimiter. In particular rgb(r,g,b,a) ignores its fourth value;
  // the Kamigawa showcase templates depend on that behavior.
  const rgb = /^rgb\(\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)/.exec(value);
  if (rgb) return new MseColor(...rgb.slice(1).map(Number));
  const rgba = /^rgba\(\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)/.exec(value);
  if (rgba) return new MseColor(...rgba.slice(1).map(Number));
  if (/^#(?:[\da-f]{3}|[\da-f]{4}|[\da-f]{6}|[\da-f]{8})$/i.test(value)) {
    const hex = value.slice(1), width = hex.length <= 4 ? 1 : 2;
    const channels = [];
    for (let i = 0; i < hex.length; i += width) channels.push(parseInt(hex.slice(i, i + width), 16) * (width === 1 ? 17 : 1));
    return new MseColor(...channels);
  }
  const channels = Object.hasOwn(names, value.toLowerCase()) ? names[value.toLowerCase()] : null;
  if (channels) return new MseColor(...channels);
  return conversion(value, 'color');
}
