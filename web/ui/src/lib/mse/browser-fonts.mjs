import { MseError } from './document.mjs';
import { resolveFontFace } from './font-definition.mjs';
import { normalizeFont } from './font-normalize.mjs';
import { readFontMetadata } from './font-metadata.mjs';
let nextFamilyId = 0;

// Named families use registered files or an explicitly enabled exact local()
// lookup. Missing fonts never silently fall back to a different browser family.
export class MseBrowserFonts {
  constructor({ fontSet = globalThis.document?.fonts, FontFaceClass = globalThis.FontFace, canvas = globalThis.document?.createElement('canvas'), normalFont = null,allowSystemFonts=false,pixelsPerPoint=96/72 } = {}) {
    if(!Number.isFinite(pixelsPerPoint)||pixelsPerPoint<=0)throw new MseError('font-provider','Invalid font pixels per point');
    this.pixelsPerPoint=pixelsPerPoint;
    this.fontSet = fontSet;
    this.FontFaceClass = FontFaceClass;
    this.context = canvas?.getContext('2d');
    this.families = new Map();
    this.normalFont = normalFont;
    this.allowSystemFonts=allowSystemFonts;
    this.localFonts=new Map();
    this.disposed = false;
  }
  requireActive() {
    if (this.disposed) throw new MseError('font-disposed', 'Font provider has been disposed');
  }
  systemFont(){
    if(!this.normalFont){
      const doc=globalThis.document;
      if(!doc?.body||!globalThis.getComputedStyle)throw new MseError('font-provider','A system UI font definition is required');
      const probe=doc.createElement('span');
      probe.style.cssText='position:absolute;visibility:hidden;font:message-box';doc.body.append(probe);
      try{const style=getComputedStyle(probe);this.normalFont={family:style.fontFamily,pixelSize:parseFloat(style.fontSize)};}
      finally{probe.remove();}
    }
    if(!this.normalFont.family||!Number.isFinite(this.normalFont.pixelSize)||this.normalFont.pixelSize<=0)throw new MseError('font-provider','Invalid system UI font definition');
    return this.normalFont;
  }
  async localFont(name){
    this.requireActive();
    if(/[\x00-\x1f\x7f]/.test(name))throw new MseError('font-missing','Invalid local font name');
    const key=name.toLowerCase();
    if(!this.localFonts.has(key))this.localFonts.set(key,this.register(name,`local(${JSON.stringify(name)})`).catch(()=>{
      this.requireActive();
      throw new MseError('font-missing',`Font is not installed or accessible: ${name}`);
    }));
    return this.localFonts.get(key);
  }
  async registerFile(bytes) {
    this.requireActive();
    const metadata = readFontMetadata(bytes);
    const normalized = normalizeFont(bytes);
    const names=new Set([...metadata.families,metadata.fullName,metadata.postscriptName].filter(Boolean));
    for (const name of names) await this.register(name, normalized.bytes, { weight: String(metadata.weight), style: metadata.style },metadata.decoration,metadata.verticalMetrics);
    return { ...metadata, repairs: normalized.repairs };
  }
  async register(name, source, descriptors = {}, decoration = null, verticalMetrics = null) {
    this.requireActive();
    if (!this.fontSet || !this.FontFaceClass) throw new MseError('font-provider', 'Browser FontFace support is required');
    const key=name.toLowerCase();
    let family = this.families.get(key);
    if (!family) {
      family = { alias: `MSEFont${nextFamilyId++}`, pending: new Set(), faces: [], metrics: [] };
      this.families.set(key, family);
    }
    const face = new this.FontFaceClass(family.alias, source, descriptors);
    const pending = face.load();
    family.pending.add(pending);
    try { await pending; this.requireActive(); this.fontSet.add(face); family.faces.push(face);family.metrics.push({weight:Number(descriptors.weight)||400,style:descriptors.style||'normal',decoration,verticalMetrics}); }
    finally { family.pending.delete(pending); }
    return face;
  }
  async prepare(font, { pixelsPerPoint = this.pixelsPerPoint, ...flags } = {}) {
    this.requireActive();
    if(!Number.isFinite(pixelsPerPoint)||pixelsPerPoint<=0)throw new MseError('font-provider','Invalid font pixels per point');
    if (!this.context) throw new MseError('font-provider', 'A canvas text context is required');
    const face = resolveFontFace(font, flags);
    let family,normal=null;
    if (!face.name) {
      // FontRef::toWxFont uses wxNORMAL_FONT for an empty family. CSS's
      // message-box system font is the browser host's corresponding UI font.
      // Hosts can inject normalFont (CSS family and pixel size) explicitly.
      normal=this.systemFont();
    } else {
      family=this.families.get(face.name.toLowerCase());
      if(!family?.faces.length&&this.localFonts.has(face.name.toLowerCase()))await this.localFonts.get(face.name.toLowerCase());
      else if(family)await Promise.all(family.pending);
      if((!family||!family.faces.length)&&this.allowSystemFonts){
        await this.localFont(face.name);family=this.families.get(face.name.toLowerCase());
      }
      if (!family) throw new MseError('font-missing', `Font is not registered: ${face.name}`);
      this.requireActive();
      if (!family.faces.length) throw new MseError('font-missing', `Font failed to load: ${face.name}`);
    }
    const ctx = this.context;
    const selectedFace=family?.metrics.slice().sort((a,b)=>{
      const rank=m=>(m.style===(face.italic?'italic':'normal')?0:10000)+Math.abs(m.weight-(face.bold?700:400));
      return rank(a)-rank(b);
    })[0];
    const selected=selectedFace?.decoration;
    const vertical=selectedFace?.verticalMetrics;
    // Native macOS exports use 72 DPI and a 4x text surface. wxGCDC
    // rounds CoreText extents upward before MSE divides them by four.
    const textScale=pixelsPerPoint===1?4:1;
    const setup = (scale, target = ctx) => {
      this.requireActive();
      const pixels = font.code&&Math.floor(textScale*scale*font.size+0.4999995)<2?this.systemFont().pixelSize/textScale
        :Math.max(1, Math.floor(textScale * scale * (normal && font.size <= 1 ? normal.pixelSize : font.size * pixelsPerPoint) + (textScale===4?0.4999995:0.5)))/textScale;
      target.font = normal ? `normal normal ${pixels}px ${normal.family}`
        : `${face.italic ? 'italic' : 'normal'} ${face.bold ? 'bold' : 'normal'} ${pixels}px "${family.alias}"`;
      target.fontKerning = 'normal';
      target.textBaseline = 'alphabetic';
      return pixels;
    };
    const extent = (text, scale) => {
      const pixels=setup(scale);
      const metric = ctx.measureText(text);
      // CoreText reports a positive descent even for legacy fonts whose hhea
      // descender is positive (the pack's ModMatrix is one such font).
      const height = textScale===4&&vertical
        ? pixels*(vertical.ascent+Math.abs(vertical.descent)+vertical.lineGap)/vertical.unitsPerEm
        : metric.fontBoundingBoxAscent + metric.fontBoundingBoxDescent;
      if (!Number.isFinite(height)) throw new MseError('font-metrics', 'Browser font bounding-box metrics are required');
      return textScale===4?{width:Math.ceil(metric.width*4)/4,height:Math.ceil(height*4)/4}:{ width: metric.width, height };
    };
    return {
      measure: (text, unusedFont, scale) => extent(text, scale),
      lineHeight: (unusedFont, scale) => extent('M', scale).height,
      draw: (target, text, x, y, scale) => {
        const pixels=setup(scale, target);
        const metrics = target.measureText(text);
        const baseline=y+metrics.fontBoundingBoxAscent;
        target.fillText(text, x, baseline);
        const unit=selected?.unitsPerEm>0?pixels/selected.unitsPerEm:null;
        // OpenType post/OS/2 positions describe the top edge, not the center.
        const line=(position,thickness)=>target.fillRect(x,baseline-position,metrics.width,thickness);
        // wxNORMAL_FONT ignores underline for an empty family, but applies
        // strikethrough. Named faces carry both decorations through wxFont.
        if(font.underline&&!normal)line(unit!==null&&Number.isFinite(selected.underlinePosition)?selected.underlinePosition*unit:-pixels/10,
          Math.max(1,unit!==null&&selected.underlineThickness>0?selected.underlineThickness*unit:pixels/20));
        if(font.strikethrough)line(unit!==null&&Number.isFinite(selected.strikePosition)?selected.strikePosition*unit:pixels*0.3,
          Math.max(1,unit!==null&&selected.strikeThickness>0?selected.strikeThickness*unit:pixels/20));
      },
    };
  }
  dispose() {
    this.disposed = true;
    for (const family of this.families.values()) for (const face of family.faces) this.fontSet.delete(face);
    this.families.clear();
    this.localFonts.clear();
  }
}
