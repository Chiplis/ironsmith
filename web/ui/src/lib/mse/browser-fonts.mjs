import { MseError } from './document.mjs';
import { resolveFontFace } from './font-definition.mjs';
import { normalizeFont } from './font-normalize.mjs';
import { readFontMetadata } from './font-metadata.mjs';
let nextFamilyId = 0;

// Families must be explicitly registered. A missing bundled font must not
// silently fall back to the browser's default and change the card's layout.
export class MseBrowserFonts {
  constructor({ fontSet = globalThis.document?.fonts, FontFaceClass = globalThis.FontFace, canvas = globalThis.document?.createElement('canvas') } = {}) {
    this.fontSet = fontSet;
    this.FontFaceClass = FontFaceClass;
    this.context = canvas?.getContext('2d');
    this.families = new Map();
    this.disposed = false;
  }
  requireActive() {
    if (this.disposed) throw new MseError('font-disposed', 'Font provider has been disposed');
  }
  async registerFile(bytes) {
    this.requireActive();
    const metadata = readFontMetadata(bytes);
    const normalized = normalizeFont(bytes);
    for (const name of metadata.families) await this.register(name, normalized.bytes, { weight: String(metadata.weight), style: metadata.style });
    return { ...metadata, repairs: normalized.repairs };
  }
  async register(name, source, descriptors = {}) {
    this.requireActive();
    if (!this.fontSet || !this.FontFaceClass) throw new MseError('font-provider', 'Browser FontFace support is required');
    let family = this.families.get(name);
    if (!family) {
      family = { alias: `MSEFont${nextFamilyId++}`, pending: new Set(), faces: [] };
      this.families.set(name, family);
    }
    const face = new this.FontFaceClass(family.alias, source, descriptors);
    const pending = face.load();
    family.pending.add(pending);
    try { await pending; this.requireActive(); this.fontSet.add(face); family.faces.push(face); }
    finally { family.pending.delete(pending); }
    return face;
  }
  async prepare(font, { pixelsPerPoint = 96 / 72, ...flags } = {}) {
    this.requireActive();
    if (!this.context) throw new MseError('font-provider', 'A canvas text context is required');
    const face = resolveFontFace(font, flags), family = this.families.get(face.name);
    if (!family) throw new MseError('font-missing', `Font is not registered: ${face.name}`);
    await Promise.all(family.pending);
    this.requireActive();
    if (!family.faces.length) throw new MseError('font-missing', `Font failed to load: ${face.name}`);
    const ctx = this.context;
    const setup = (scale, target = ctx) => {
      this.requireActive();
      const pixels = Math.max(1, Math.floor(font.size * scale * pixelsPerPoint + 0.5));
      target.font = `${face.italic ? 'italic' : 'normal'} ${face.bold ? 'bold' : 'normal'} ${pixels}px "${family.alias}"`;
      target.fontKerning = 'normal';
      target.textBaseline = 'alphabetic';
    };
    const extent = (text, scale) => {
      setup(scale);
      const metric = ctx.measureText(text);
      const height = metric.fontBoundingBoxAscent + metric.fontBoundingBoxDescent;
      if (!Number.isFinite(height)) throw new MseError('font-metrics', 'Browser font bounding-box metrics are required');
      return { width: metric.width, height };
    };
    return {
      measure: (text, unusedFont, scale) => extent(text, scale),
      lineHeight: (unusedFont, scale) => extent('M', scale).height,
      draw: (target, text, x, y, scale) => {
        setup(scale, target);
        const metrics = target.measureText(text);
        target.fillText(text, x, y + metrics.fontBoundingBoxAscent);
      },
    };
  }
  dispose() {
    this.disposed = true;
    for (const family of this.families.values()) for (const face of family.faces) this.fontSet.delete(face);
    this.families.clear();
  }
}
