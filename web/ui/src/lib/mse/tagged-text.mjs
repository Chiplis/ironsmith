import { color } from './color-values.mjs';
import { MseError } from './document.mjs';
import { measureTextRun } from './text-metrics.mjs';
import { measureSymbolRun } from './symbol-layout.mjs';
import { splitSymbols } from './symbol-font.mjs';

export function parseTaggedText(source, baseFont) {
  const decorations = [], activeDecorations = [];
  const runs = [], unsupported = new Set(), warnings = [];
  const counts = Object.fromEntries(['b','i','u','strike','sym','line','soft-line','soft','sep-soft','atom-kwpph'].map(k => [k, 0]));
  const stacks = { color: [], size: [], font: [] };
  const append = (content, start, end) => {
    if (!content) return;
    const soft = counts.soft > 0 || counts['sep-soft'] > 0;
    const font = { ...baseFont, bold: baseFont.bold || counts.b > 0, italic: baseFont.italic || counts.i > 0,
      underline: baseFont.underline || counts.u > 0, strikethrough: baseFont.strikethrough || counts.strike > 0 };
    if (soft || counts['atom-kwpph'] > 0) { font.color = font.separator_color; font.shadow_displacement_x = 0; font.shadow_displacement_y = 0; }
    if (stacks.color.length) font.color = stacks.color.at(-1);
    if (stacks.size.length) font.size = stacks.size.at(-1);
    if (stacks.font.length && stacks.font.at(-1)) font.name = stacks.font.at(-1);
    // Native placeholders occupy the adjacent markup characters with visible
    // angle brackets; they use separator color but remain normal drawn text.
    if (counts['atom-kwpph'] > 0 && counts.sym <= 0) { content='‹'+content+'›';start--;end++; }
    runs.push({ text: content.replaceAll('\u0001','<'), start, end, font, fromTag: stacks.font.length > 0,
      decorationIds: activeDecorations.map(d => d.id), symbol: counts.sym > 0, soft, breakStyle: counts.line > 0 ? 'line' : counts['soft-line'] > 0 ? 'soft' : 'hard' });
  };
  let pos = 0;
  for (const match of source.matchAll(/<[^>]*(?:>|$)/g)) {
    append(source.slice(pos, match.index), pos, match.index);
    pos = match.index + match[0].length;
    const tag = match[0].slice(1).replace(/>$/, ''), closing = tag.startsWith('/'), body = closing ? tag.slice(1) : tag;
    const counter = Object.keys(counts).sort((a,b) => b.length-a.length).find(k => body === k || body.startsWith(k+':') || body.startsWith(k+'-') || body.startsWith(k+' '));
    if (counter) { counts[counter] += closing ? -1 : 1; continue; }
    const stackName = Object.keys(stacks).find(k => body === k || body.startsWith(k+':') || body.startsWith(k+'-') || body.startsWith(k+' '));
    if (stackName) {
      if (closing) stacks[stackName].pop();
      else if (body.includes(':')) {
        let value = body.slice(body.indexOf(':') + 1);
        if (stackName === 'color') {
          try { value = color(value); } catch { warnings.push(`Invalid color: ${value}`); value = baseFont.color; }
        } else if (stackName === 'size') { value = Number(value); if (!Number.isFinite(value)) value = baseFont.size; }
        stacks[stackName].push(value);
      }
      continue;
    }
    const wrapper = /^(atom|error)(?:$|[-: ])/.exec(body)?.[1];
    if (wrapper && !/^atom-(param|kwpph)(?:$|[-: ])/.test(body)) {
      if (closing) {
        const index = activeDecorations.findLastIndex(d => d.type === wrapper);
        if (index >= 0) {
          activeDecorations[index].end = match.index;
          activeDecorations.splice(index, 1);
        }
      } else {
        const dark = baseFont.color.channels.slice(0,3).reduce((a,b) => a+b,0) < 510;
        const decoration = { id: decorations.length, type: wrapper, start: pos, end: source.length,
          ...(wrapper === 'atom' ? { background: color(dark ? 'rgb(210,210,210)' : 'rgb(60,60,60)') } : {}) };
        decorations.push(decoration); activeDecorations.push(decoration);
      }
      continue;
    }
    // These native tags need structural layout or editor decorations. Do not
    // report a successful measurement while silently discarding their effect.
    if (/^(?:atom|error|li|bullet|margin|align|code|ref-param)(?:$|[-: ])/.test(body)) unsupported.add(body.split(/[: ]/)[0]);
  }
  append(source.slice(pos), pos, source.length);
  const indices = new Map();
  let utf16 = 0, character = 0;
  for (const c of source) { indices.set(utf16, character++); utf16 += c.length; }
  indices.set(source.length, character);
  for (const item of [...runs, ...decorations]) { item.sourceStart = indices.get(item.start); item.sourceEnd = indices.get(item.end); }
  return { runs, decorations, sourceLength: character, unsupported: [...unsupported], warnings };
}

export async function measureTaggedText(parsed, fonts, symbolRun, renderer, { scale = 1, alwaysSymbol = false } = {}) {
  if (parsed.unsupported.length) throw new MseError('text-tags', `Unsupported structural text tags: ${parsed.unsupported.join(', ')}`);
  const runs = [], characters = Array.from({ length: parsed.sourceLength }, () => ({ width: 0, height: 0, break: 'no', soft: true, bullet: false, tag: true }));
  for (const run of parsed.runs) {
    const segments = [];
    if (symbolRun?.font && (run.symbol || alwaysSymbol)) {
      const matches = splitSymbols(symbolRun.font, run.text);
      if (run.symbol) segments.push({ ...run, matches, symbol: true });
      else {
        let offset = 0;
        for (const match of matches) {
          if (match.offset > offset) segments.push({ ...run, text: run.text.slice(offset, match.offset), sourceStart: run.sourceStart + [...run.text.slice(0, offset)].length, symbol: false });
          segments.push({ ...run, text: match.text, sourceStart: run.sourceStart + [...run.text.slice(0, match.offset)].length, matches: [match], symbol: true });
          offset = match.offset + match.text.length;
        }
        if (offset < run.text.length) segments.push({ ...run, text: run.text.slice(offset), sourceStart: run.sourceStart + [...run.text.slice(0, offset)].length, symbol: false });
      }
    } else segments.push({ ...run, symbol: false });
    for (const segment of segments) {
      const measured = segment.symbol
        ? await measureSymbolRun({ ...symbolRun, symbols: segment.matches }, renderer, { scale })
        : measureTextRun(segment.text, segment.font, await fonts.prepare(segment.font, { fromTag: segment.fromTag }), { scale, soft: segment.soft, breakStyle: segment.breakStyle });
      runs.push({ ...segment, characterStart: segment.sourceStart, measured });
      measured.characters.forEach((c, i) => { characters[segment.sourceStart + i] = c; });
    }
  }
  return { runs, characters, decorations: parsed.decorations, warnings: parsed.warnings,
    minScale: Math.max(0.0001, ...runs.map(r => r.measured.minScale)),
    scaleStep: Math.min(1, ...runs.map(r => r.measured.scaleStep)),
  };
}
