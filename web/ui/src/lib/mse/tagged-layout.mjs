import { alignmentOffset } from './alignment.mjs';
import { measureTaggedText } from './tagged-text.mjs';
import { selectTextScaleAsync } from './text-scale.mjs';
import { wrapTextCharacters } from './text-wrap.mjs';
import { alignTextLines, textContentHeight } from './text-alignment.mjs';

export async function layoutTaggedText(parsed, fonts, symbols, renderer, options) {
  const { width, height, paddingLeft = 0, paddingRight = 0, paddingTop = 0, paddingBottom = 0, previousScale = 1 } = options;
  const measured = new Map();
  const atScale = async scale => {
    if (!measured.has(scale)) measured.set(scale, await measureTaggedText(parsed, fonts, symbols, renderer, { scale, alwaysSymbol: options.alwaysSymbol }));
    return measured.get(scale);
  };
  const initial = await atScale(previousScale);
  const result = await selectTextScaleAsync(async scale => {
    const measurement = await atScale(scale);
    return { ...wrapTextCharacters(measurement.characters, options), measurement };
  }, { minScale: initial.minScale, scaleStep: initial.scaleStep, previousScale, height, paddingBottom });
  let wrapped = result;
  let offset = 0;
  const seen = [];
  for (let iteration = 0; iteration < 6; iteration++) {
    if (!(options.paragraphHeight <= 0 || options.paragraphHeight === undefined) || !(options.alignment & (512 | 1024))) break;
    const next = alignmentOffset(options.alignment, 0, height - paddingTop - paddingBottom, 0, textContentHeight(wrapped.lines)).y;
    if (Math.abs(next - offset) < 0.5 || seen.some(value => Math.abs(value - next) < 0.5)) break;
    seen.push(next); offset = next;
    wrapped = wrapTextCharacters(result.measurement.characters, { ...options, topOffset: offset });
  }
  const lines = alignTextLines(wrapped.lines, result.measurement.characters, {
    ...options, width: width - paddingLeft - paddingRight, height: height - paddingTop - paddingBottom,
    maskLeft: paddingLeft, maskRight: width - paddingRight, y: paddingTop,
  });
  return { ...result, lines, width: Math.max(0, ...lines.map(l => l.width)), height: textContentHeight(lines) };
}
