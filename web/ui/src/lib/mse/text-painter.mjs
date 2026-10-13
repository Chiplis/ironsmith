import { cssColor, requirePlainFont, prepareSymbolDrawing } from './symbol-painter.mjs';

// Drawing consumes the same source-indexed runs and positions as measurement.
// Unsupported effects fail before drawing so callers cannot mistake a partial
// frame for a complete renderer result.
export async function paintTextLayout(ctx, layout, fonts, { x = 0, y = 0, showSoft = false, createBitmap, fontSizeStep } = {}) {
  const runs = layout.measurement.runs, prepared = new Map();
  for (const run of runs) {
    if (run.symbol) {
      prepared.set(run, await prepareSymbolDrawing(run.measured, fonts, { createBitmap, fontSizeStep }));
      continue;
    }
    const font = run.font;
    requirePlainFont(font);
    prepared.set(run, await fonts.prepare(font, { fromTag: run.fromTag }));
  }
  ctx.save();
  try {
    ctx.translate(x, y);
    for (const line of layout.lines) for (const run of runs) {
      if (run.soft && !showSoft) continue;
      if (run.symbol) { prepared.get(run)(ctx, line, run.characterStart); continue; }
      const chars = [...run.text];
      const start = Math.max(line.start, run.characterStart), end = Math.min(line.end, run.characterStart + chars.length);
      if (start >= end) continue;
      ctx.fillStyle = cssColor(run.font.color);
      const draw = (a,b) => {
        const content = chars.slice(a-run.characterStart,b-run.characterStart).join('').replace(/\n$/, '');
        if (content) prepared.get(run).draw(ctx, content, line.positions[a-line.start], line.top, layout.scale);
      };
      if (line.justifying) for (let i=start;i<end;i++) draw(i,i+1);
      else draw(start,end);
    }
  } finally { ctx.restore(); }
}
