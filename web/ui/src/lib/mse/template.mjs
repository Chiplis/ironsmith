import { createTextMask } from './text-mask.mjs';
import { conformRaster } from './image-conform.mjs';
import { readSymbolVariation } from './symbol-variation.mjs';
import { textLayoutInfo, dummyTextLayout } from './text-layout-info.mjs';
import { layoutTaggedText } from './tagged-layout.mjs';
import { parseTaggedText, measureTaggedText } from './tagged-text.mjs';
import { selectTextScale } from './text-scale.mjs';
import { alignTextLines, textContentHeight } from './text-alignment.mjs';
import { parseAlignment } from './alignment.mjs';
import { wrapTextCharacters } from './text-wrap.mjs';
import { measureTextRun } from './text-metrics.mjs';
import { measureSymbolRun } from './symbol-layout.mjs';
import { loadSymbolFont, evaluateSymbolFont, splitSymbols } from './symbol-font.mjs';
import { compileFontDefinition, evaluateFontDefinition } from './font-definition.mjs';
import { geometryNames, geometryDefaults, geometryModes, geometryValue } from './style-geometry.mjs';
import { canonicalName, MseError } from './document.mjs';
import { MseScriptLoader } from './script-loader.mjs';
import { MseRuntime, MseScope } from './script-runtime.mjs';
import { coreBuiltins } from './script-builtins.mjs';
import { MseObject, MseSet, MseFieldValue, fromHost, force, boolean, number, text, missingMember } from './script-values.mjs';
import { color } from './color-values.mjs';
import { loadMseKeywords, keywordBuiltins } from './keywords.mjs';
import { imageBuiltins, mseImage } from './image-builtins.mjs';
import { MseCardRegistry, cardLinkBuiltins } from './card-links.mjs';
import { choiceDefinitions, normalizeMultipleChoice } from './choice-fields.mjs';

const numericStyleProperties = new Set('left right top bottom width height angle z_index tab_index padding_left padding_right padding_top padding_bottom padding_left_min padding_right_min padding_top_min padding_bottom_min line_height_soft line_height_hard line_height_line line_height_soft_max line_height_hard_max line_height_line_max paragraph_height'.split(' '));

function initialValue(record, choices) {
  const value = record.text('initial');
  switch (record.text('type')) {
    case 'boolean': return value || 'yes';
    case 'choice': return value || choices[0]?.name || '';
    case 'slider': return Number(value || 0);
    case 'color': return color(value || 'black');
    case 'image': case 'symbol': return null;
    default: return value;
  }
}

// A definition retains every field/property record. Only the schema consumer
// decides whether a nested block is a script, a font, or a repeated choice.
export async function loadMseTemplate(store, name, { setKeywords = [] } = {}) {
  const pkg = await store.package(name);
  const style = await store.expandRecord(pkg.document);
  const gamePackage = await store.package(style.text('game') + '.mse-game');
  const game = await store.expandRecord(gamePackage.document);
  const loader = new MseScriptLoader(store);
  const compile = entry => entry ? loader.compile(entry.text(), { ...entry.location, line: entry.location.line + (entry.inline ? 0 : 1) }) : null;
  const definitions = { card: new Map(), set: new Map(), styling: new Map(), extra_card: new Map() };
  for (const [group, records, key] of [
    ['card', [game, style], 'card field'], ['set', [game], 'set field'], ['styling', [style], 'styling field'],
    ['extra_card', [style], 'extra card field'],
  ]) for (const source of records) for (const entry of source.all(key)) {
    const record = await store.expandRecord(entry.record());
    const fieldName = canonicalName(record.text('name'));
    if (!fieldName || record.text('type') === 'info') continue;
    const choices = ['choice', 'multiple choice', 'boolean'].includes(record.text('type')) ? choiceDefinitions(record) : [];
    definitions[group].set(fieldName, {
      name: fieldName, record, choices, initial: initialValue(record, choices),
      defaultScript: await compile(record.last('default')),
      script: await compile(record.last('script')),
    });
  }
  const layouts = new Map(), extraLayouts = new Map();
  for (const [recordName, target] of [['card style', layouts], ['extra card style', extraLayouts]]) for (const entry of style.all(recordName)) {
    const record = await store.expandRecord(entry.record());
    for (const field of record.entries) {
      const properties = await store.expandRecord(field.record());
      const programs = new Map(), choiceImages = new Map();
      for (const choice of properties.last('choice images')?.record().entries || []) {
        const script = !choice.inline ? choice.record().last('script') : null;
        choiceImages.set(choice.key, script ? await compile(script) : await loader.compile(choice.text(), { ...choice.location, stringMode: true }));
      }
      for (const property of properties.entries) {
        // Literal nested objects (font, symbol font, choice images) are kept as
        // records. Scalar properties are interpolated using MSE string mode.
        if (!property.inline && property.block.length && !property.text().trimStart().startsWith('{')) {
          const script = property.record().last('script');
          if (script) programs.set(property.key, await compile(script));
          continue;
        }
        programs.set(property.key, await loader.compile(property.text(), { ...property.location, stringMode: true }));
      }
      target.set(field.key, { record: properties, programs, choiceImages, geometry: geometryModes(properties), font: await compileFontDefinition(properties.last('font')?.record(), loader), symbolFont: await compileFontDefinition(properties.last('symbol font')?.record(), loader, { symbol: true }) });
    }
  }
  const init = [];
  const symbolVariations = new Map();
  for (const [source,key] of [[game,'default set style'],[style,'set info style']]) {
    for (const block of source.all(key)) for (const field of (await store.expandRecord(block.record())).entries) {
      const record = await store.expandRecord(field.record());
      for (const entry of record.all('variation')) {
        const variation = readSymbolVariation(await store.expandRecord(entry.record()));
        if (!symbolVariations.has(field.key)) symbolVariations.set(field.key,new Map());
        symbolVariations.get(field.key).set(variation.name,variation);
      }
    }
  }
  for (const source of [game, style]) for (const entry of source.all('init script')) init.push(await compile(entry));
  const keywords = await loadMseKeywords(game, loader, setKeywords);
  const faceScripts = { front: await compile(game.last('get front face script')), back: await compile(game.last('get back face script')) };
  const linkAliases = new Map();
  for (const entry of game.all('card link')) {
    const record = entry.record();
    for (const side of ['selected', 'linked']) for (const alias of record.last(`localized ${side}`)?.record().entries || []) linkAliases.set(alias.text(), record.text(side));
  }
  return { name, store, game, style, definitions, layouts, extraLayouts, init, keywords, faceScripts, linkAliases, symbolVariations };
}

export class MseTemplateContext {
  constructor(template, { card = {}, set = {}, styling = {}, extraCard = {}, builtins = {}, spellcheck, spellcheckEnabled, uid = card.uid, cardRegistry = new MseCardRegistry() } = {}) {
    this.template = template;
    this.active = false;
    this.reading = [];
    this.cells = {};
    this.layoutValues = new Map();
    this.textMeasurements = null;
    this.layoutRevision = 0;
    this.keywordUsage = new Map();
    this.cardRegistry = cardRegistry;
    this.runtime = new MseRuntime({ builtins: { ...coreBuiltins({ spellcheck, spellcheckEnabled }), ...keywordBuiltins(template.keywords, this.keywordUsage, () => this.reading.at(-1) || 'input'), ...imageBuiltins(template.name), ...cardLinkBuiltins(cardRegistry), exists_in_package: scope => template.store.exists(text(scope.get('input'))), ...builtins } });
    for (const [group, input] of Object.entries({ card, set, styling, extra_card: extraCard })) {
      const cells = new Map(); this.cells[group] = cells;
      const supplied = Object.fromEntries(Object.entries(input).map(([key, value]) => [canonicalName(key), fromHost(value)]));
      for (const [name, definition] of template.definitions[group]) {
        const value = Object.hasOwn(supplied, name) ? supplied[name] : definition.initial;
        cells.set(name, { definition, value, sourceValue: value, isDefault: !Object.hasOwn(supplied, name), resolved: false });
      }
      for (const [name, value] of Object.entries(supplied)) if (!cells.has(name)) cells.set(name, { value, isDefault: false, resolved: true });
      this.runtime.scope.set(group, new MseObject(name => this.field(group, name)));
    }
    this.runtime.scope.set('game', fromHost({ name: template.style.text('game') }));
    this.runtime.scope.set('stylesheet', fromHost({ name: template.name, card_width: Number(template.style.text('card width')), card_height: Number(template.style.text('card height')) }));
    this.runtime.scope.set('card_style', new MseObject(name => template.layouts.has(name) ? new MseObject(property => this.property(name, property)) : missingMember(name)));
    this.runtime.scope.set('extra_card_style', new MseObject(name => template.extraLayouts.has(name) ? new MseObject(property => this.property(name, property, 'extra_card')) : missingMember(name)));
    this.card = cardRegistry.register(this, card, uid);
    this.runtime.scope.set('card', this.card);
    this.runtime.scope.set('set', new MseSet(cardRegistry, name => this.field('set', name)));
    this.cells.set.set('cards', { value: cardRegistry.cards, resolved: true, isDefault: false });
  }
  initialize() {
    if (this.active || this.initializing) return this;
    this.initializing = true;
    try { for (const program of this.template.init) this.runtime.run(program); this.active = true; }
    finally { this.initializing = false; }
    return this;
  }
  invalidate({ keepMeasurements = false } = {}) {
    this.layoutRevision++;
    if (!keepMeasurements) this.textMeasurements = null;
    this.layoutValues.clear(); this.keywordUsage.clear();
    for (const [group, cells] of Object.entries(this.cells)) for (const [name, cell] of cells) {
      if (cell.definition && !this.reading.includes(`${group}.${name}`)) { cell.value = cell.sourceValue; cell.resolved = false; }
    }
  }
  setField(group, name, value, { age = ++this.cardRegistry.age, isDefault = false, lastChange = '' } = {}) {
    name = canonicalName(name);
    const cell = this.cells[group]?.get(name);
    if (!cell) throw new MseError('field-name', `No field ${group}.${name}`);
    value = fromHost(value);
    this.cardRegistry.age = Math.max(this.cardRegistry.age, age);
    Object.assign(cell, { value, sourceValue: value, isDefault, lastUpdate: age, lastChange, resolved: false });
    this.cardRegistry.changed();
  }
  fieldValue(group, name, cell) {
    return new MseFieldValue(cell.value, cell.isDefault, { cell, context: this, group, name });
  }
  field(group, name) {
    const cell = this.cells[group].get(name);
    if (!cell) return missingMember(`${group}.${name}`);
    const key = `${group}.${name}`;
    if (this.reading.includes(key) && this.reading.at(-1) !== key) throw new MseError('field-cycle', `Circular field dependency ${[...this.reading, key].join(' -> ')}`);
    if (this.active && !cell.resolved && !this.reading.includes(key)) {
      this.reading.push(key);
      try {
        const scope = new MseScope(this.runtime.scope);
        scope.set('value', this.fieldValue(group, name, cell));
        scope.set('input', cell.value);
        const multipleChoice = cell.definition.record.text('type') === 'multiple choice';
        if (multipleChoice) scope.set('last_change', cell.lastChange || '');
        if (cell.isDefault && cell.definition.defaultScript) cell.value = force(this.runtime.evaluate(cell.definition.defaultScript, scope));
        scope.set('input', cell.value);
        if (cell.definition.script) cell.value = force(this.runtime.evaluate(cell.definition.script, scope));
        if (multipleChoice) cell.value = normalizeMultipleChoice(text(cell.value), cell.definition.choices, cell.definition.record.text('empty choice'));
        cell.resolved = true;
      } finally { this.reading.pop(); }
    }
    return this.fieldValue(group, name, cell);
  }
  async prepareTextLayouts(fonts, renderer, { maxPasses = 16 } = {}) {
    const generation = this.textPreparationGeneration = (this.textPreparationGeneration || 0) + 1;
    const fields = [];
    for (const [group, layouts] of [['card', this.template.layouts], ['extra_card', this.template.extraLayouts]]) {
      for (const name of layouts.keys()) {
        const type = this.cells[group].get(name)?.definition?.record.text('type');
        if (['text','choice','boolean','multiple choice'].includes(type)) fields.push({ group, name, type, key: `${group}.${name}` });
      }
    }
    this.textMeasurements = new Map(fields.map(field => [field.key, { info: dummyTextLayout(), result: null }]));
    this.invalidate({ keepMeasurements: true });
    let previous = '';
    try {
      for (let pass = 1; pass <= maxPasses; pass++) {
        const revision = this.layoutRevision, next = new Map();
        for (const field of fields) {
          if (!this.fieldVisible(field.name,field.group)) {
            next.set(field.key,this.textMeasurements.get(field.key));
            continue;
          }
          const result = field.type === 'text'
            ? await this.layoutTaggedText(field.name, this.field(field.group, field.name), fonts, renderer, { group: field.group, previousScale: this.textMeasurements.get(field.key).result?.scale || 1 })
            : await this.measureChoiceImage(field.name, renderer, field.group);
          if (generation !== this.textPreparationGeneration || revision !== this.layoutRevision) throw new MseError('layout-stale', 'Card changed or text layout preparation was superseded');
          next.set(field.key, { info: field.type === 'text' ? textLayoutInfo(result.lines) : { width: result.width, height: result.height, lines: [] }, result });
        }
        const signature = JSON.stringify([...next].map(([key, value]) => [key, value.info, value.result?.scale, value.result?.lines]));
        this.textMeasurements = next;
        this.invalidate({ keepMeasurements: true });
        if (signature === previous) return { passes: pass, fields: fields.length, measurements: next, renderingVerified: false };
        previous = signature;
      }
      throw new MseError('layout-convergence', `Text layout did not stabilize after ${maxPasses} passes`);
    } catch (error) {
      // Font/image loading may finish after another preparation has completed.
      // A superseded request must not clear that newer request's measurements.
      if (generation === this.textPreparationGeneration) this.invalidate();
      throw error;
    }
  }
  fieldVisible(field,group='card') {
    const layout=(group==='card'?this.template.layouts:this.template.extraLayouts).get(field);
    if(layout.programs.has('visible')&&!boolean(this.property(field,'visible',group)))return false;
    return this.property(field,'width',group)>0 && this.property(field,'height',group)>0
      && Math.abs(this.property(field,'left',group))<100000 && Math.abs(this.property(field,'top',group))<100000;
  }
  async measureChoiceImage(field, renderer, group = 'card') {
    const layout = (group === 'card' ? this.template.layouts : this.template.extraLayouts).get(field);
    const get = (key, fallback) => layout.programs.has(key) ? this.property(field,key,group) : fallback;
    const renderStyle = text(get('render_style','text'));
    const empty = { width: 0, height: 0, lines: [], scale: 1, image: null };
    if (!renderStyle.includes('image') || (!layout.programs.has('image') && !layout.choiceImages.size)) return empty;
    const graph = mseImage(this.property(field,'image',group),this.template.name);
    if (graph.operation === 'blank') return empty;
    const bounds = renderStyle.includes('text') ? {width:0,height:0} : {
      width:Math.trunc(this.property(field,'width',group)),height:Math.trunc(this.property(field,'height',group)),
    };
    const raster = await renderer.render(graph,bounds);
    const image = conformRaster(raster,{...bounds,stretch:Boolean(parseAlignment(text(get('alignment','stretch')))&16)},renderer.maxPixels);
    return {width:image.width,height:image.height,lines:[],scale:1,image};
  }
  async layoutTaggedText(field, content, fonts, renderer, { group = 'card', previousScale = 1 } = {}) {
    const layout = (group === 'card' ? this.template.layouts : this.template.extraLayouts).get(field);
    const get = (key, fallback) => layout.programs.has(key) ? this.property(field, key, group) : fallback;
    const parsed = parseTaggedText(text(content), this.font(field, group));
    const symbols = await this.symbols(field, '', group);
    const width = this.property(field, 'width', group), height = this.property(field, 'height', group);
    const mask = get('mask', null);
    let rowBounds = null;
    if (mask !== null && mask !== '') {
      const image = mseImage(mask, this.template.name);
      if (image.operation !== 'blank') rowBounds = createTextMask(await renderer.render(image, { width: Math.ceil(width), height: Math.ceil(height) }), width, height);
    }
    return layoutTaggedText(parsed, fonts, symbols, renderer, {
      rowBounds,
      width: this.property(field, 'width', group), height: this.property(field, 'height', group), previousScale,
      multiLine: boolean(this.cells[group].get(field)?.definition?.record.text('multi line') || false),
      alwaysSymbol: boolean(get('always_symbol', false)), alignment: parseAlignment(text(get('alignment', 'left top'))),
      vertical: ['top to bottom', 'vertical'].includes(get('direction', '')),
      paragraphHeight: get('paragraph_height', -1),
      ...Object.fromEntries(['paddingLeft','paddingRight','paddingTop','paddingBottom','lineHeightSoft','lineHeightHard','lineHeightLine','lineHeightSoftMax','lineHeightHardMax','lineHeightLineMax'].map(name => [name, get(name.replace(/[A-Z]/g, c => '_' + c.toLowerCase()), name.startsWith('lineHeight') && !name.endsWith('Max') ? 1 : 0)])),
    });
  }
  async measureTaggedText(field, content, fonts, renderer, { group = 'card', scale = 1 } = {}) {
    const layout = (group === 'card' ? this.template.layouts : this.template.extraLayouts).get(field);
    const parsed = parseTaggedText(text(content), this.font(field, group));
    const symbols = await this.symbols(field, '', group);
    const alwaysSymbol = layout.programs.has('always_symbol') && boolean(this.property(field, 'always_symbol', group));
    return measureTaggedText(parsed, fonts, symbols, renderer, { scale, alwaysSymbol });
  }
  fitTextRun(field, content, metrics, { group = 'card', previousScale = 1, ...options } = {}) {
    const font = this.font(field, group);
    const layout = (group === 'card' ? this.template.layouts : this.template.extraLayouts).get(field);
    const result = selectTextScale(scale => this.wrapTextRun(field, content, metrics, { ...options, group, scale }), {
      minScale: Math.min(font.size, font.scale_down_to) / Math.max(0.01, font.size),
      scaleStep: 1 / Math.max(font.size * 4, 1), previousScale,
      height: this.property(field, 'height', group),
      paddingBottom: layout.programs.has('padding_bottom') ? this.property(field, 'padding_bottom', group) : 0,
    });
    return { ...this.alignTextRun(field, content, metrics, { ...options, group, scale: result.scale }), scale: result.scale };
  }
  alignTextRun(field, content, metrics, { group = 'card', ...options } = {}) {
    const measured = this.measureTextRun(field, content, metrics, { group, ...options });
    const wrapped = this.wrapTextRun(field, content, metrics, { group, ...options });
    if (!wrapped.lines.length) return wrapped;
    const layout = (group === 'card' ? this.template.layouts : this.template.extraLayouts).get(field);
    const get = (key, fallback) => layout.programs.has(key) ? this.property(field, key, group) : fallback;
    const left = get('padding_left', 0), right = get('padding_right', 0);
    const width = this.property(field, 'width', group);
    const multiLine = options.multiLine ?? boolean(this.cells[group].get(field)?.definition?.record.text('multi line') || false);
    const lines = alignTextLines(wrapped.lines, measured.characters, {
      alignment: parseAlignment(text(get('alignment', 'left top'))), multiLine,
      width: width - left - right, maskLeft: left, maskRight: width - right,
      height: this.property(field, 'height', group) - get('padding_top', 0) - get('padding_bottom', 0),
      y: get('padding_top', 0), paragraphHeight: get('paragraph_height', -1),
      lineHeightSoft: get('line_height_soft', 1), lineHeightHard: get('line_height_hard', 1), lineHeightLine: get('line_height_line', 1),
      lineHeightSoftMax: get('line_height_soft_max', 0), lineHeightHardMax: get('line_height_hard_max', 0), lineHeightLineMax: get('line_height_line_max', 0),
    });
    return { ...wrapped, lines, height: textContentHeight(lines), width: Math.max(0, ...lines.map(line => line.width)) };
  }
  wrapTextRun(field, content, metrics, { group = 'card', ...options } = {}) {
    const measured = this.measureTextRun(field, content, metrics, { group, ...options });
    const layout = (group === 'card' ? this.template.layouts : this.template.extraLayouts).get(field);
    const property = (name, fallback) => layout?.programs.has(name) ? this.property(field, name, group) : fallback;
    const definition = this.cells[group].get(field)?.definition?.record;
    return wrapTextCharacters(measured.characters, {
      width: this.property(field, 'width', group), height: this.property(field, 'height', group),
      paddingBottom: property('padding_bottom', 0), paragraphHeight: property('paragraph_height', -1),
      multiLine: boolean(definition?.text('multi line') || false),
      vertical: property('direction', '') === 'top to bottom' || property('direction', '') === 'vertical',
      paddingLeft: property('padding_left', 0), paddingRight: property('padding_right', 0), paddingTop: property('padding_top', 0),
      lineHeightSoft: property('line_height_soft', 1), lineHeightHard: property('line_height_hard', 1), lineHeightLine: property('line_height_line', 1),
      ...options,
    });
  }
  measureTextRun(field, content, metrics, { group = 'card', ...options } = {}) {
    return measureTextRun(text(content), this.font(field, group), metrics, options);
  }
  async measureSymbols(field, source, renderer, { group = 'card', scale = 1 } = {}) {
    return measureSymbolRun(await this.symbols(field, source, group), renderer, { scale });
  }
  async symbols(field, source, group = 'card') {
    const settings = this.font(field, group, true);
    const definition = await loadSymbolFont(this.template.store, settings.name);
    const scope = new MseScope(this.runtime.scope);
    scope.set('value', this.field(group, field));
    const font = evaluateSymbolFont(definition, this.runtime, scope);
    return { settings, font, symbols: splitSymbols(font, text(source)) };
  }
  font(field, group = 'card', symbol = false) {
    const key = `${group}_${symbol ? 'symbol_font' : 'font'}.${field}`;
    if (this.layoutValues.has(key)) return this.layoutValues.get(key);
    const layout = (group === 'card' ? this.template.layouts : this.template.extraLayouts).get(field);
    if (!layout) return missingMember(key);
    if (this.reading.includes(key)) throw new MseError('layout-cycle', `Circular font definition ${key}`);
    this.reading.push(key);
    try {
      const scope = new MseScope(this.runtime.scope);
      scope.set('value', this.field(group, field));
      const font = evaluateFontDefinition(symbol ? layout.symbolFont : layout.font, program => force(this.runtime.evaluate(program, scope)));
      this.layoutValues.set(key, font);
      return font;
    } finally { this.reading.pop(); }
  }
  property(field, name, group = 'card') {
    if (['content_width','content_height','content_lines','layout'].includes(name)) {
      const measured = this.textMeasurements?.get(`${group}.${field}`);
      if (measured) return name === 'layout' ? fromHost(measured.info) : name === 'content_lines' ? measured.info.lines.length : measured.info[name.slice(8)];
    }
    if (!geometryNames.has(name)) return this.rawProperty(field, name, group);
    const layout = (group === 'card' ? this.template.layouts : this.template.extraLayouts).get(field);
    if (!layout) return missingMember(`${group}_style.${field}.${name}`);
    const key = `${group}_geometry.${field}.${name}`;
    if (this.layoutValues.has(key)) return this.layoutValues.get(key);
    if (this.reading.includes(key)) throw new MseError('layout-cycle', `Circular geometry property ${key}`);
    this.reading.push(key);
    try {
      const result = geometryValue(name, key => layout.programs.has(key) ? this.rawProperty(field, key, group) : geometryDefaults[key], layout.geometry);
      this.layoutValues.set(key, result);
      return result;
    } finally { this.reading.pop(); }
  }
  rawProperty(field, name, group = 'card') {
    const key = `${group}_style.${field}.${name}`;
    if (this.layoutValues.has(key)) return this.layoutValues.get(key);
    const layout = (group === 'card' ? this.template.layouts : this.template.extraLayouts).get(field);
    let program = layout?.programs.get(name);
    if (!program && name === 'image' && layout?.choiceImages.size) {
      program = layout.choiceImages.get(canonicalName(text(this.field(group, field))));
      if (!program) return null;
    }
    if (!program) return missingMember(key);
    if (this.reading.includes(key)) throw new MseError('layout-cycle', `Circular layout property ${key}`);
    this.reading.push(key);
    try {
      const scope = new MseScope(this.runtime.scope);
      scope.set('value', this.field(group, field));
      if (name === 'image' && ['choice', 'multiple choice', 'boolean'].includes(this.cells[group].get(field)?.definition?.record.text('type'))) scope.set('input', force(this.field(group, field)));
      let result = force(this.runtime.evaluate(program, scope));
      if (numericStyleProperties.has(name)) result = number(result);
      this.layoutValues.set(key, result);
      return result;
    } finally { this.reading.pop(); }
  }
  layout({ visibleOnly = false } = {}) {
    const fields = [];
    for (const [group, layouts] of [['card', this.template.layouts], ['extra_card', this.template.extraLayouts]]) for (const [name, layout] of layouts) {
      // Rendering must not evaluate images/fonts belonging to inactive styles.
      // The full layout remains available for explicit diagnostic audits.
      if (visibleOnly && !this.fieldVisible(name, group)) continue;
      const properties = Object.create(null);
      for (const key of new Set([...layout.programs.keys(), ...geometryNames])) properties[key] = this.property(name, key, group);
      if (layout.record.last('font')) properties.font = this.font(name, group);
      if (layout.record.last('symbol font')) properties.symbol_font = this.font(name, group, true);
      fields.push({ name, group, value: this.field(group, name), properties, record: layout.record });
    }
    return {
      width: Number(this.template.style.text('card width')),
      height: Number(this.template.style.text('card height')),
      fields: fields.sort((a, b) => number(a.properties.z_index ?? 0) - number(b.properties.z_index ?? 0)),
    };
  }
}

// Construct every card before initialization so UID lookups and set.cards see
// the complete set, including cards using another stylesheet of the same game.
export class MseTemplateSet {
  constructor(template, { cards = [], set = {}, styling = {}, builtins = {}, spellcheck } = {}) {
    this.registry = new MseCardRegistry();
    this.contexts = cards.map(entry => new MseTemplateContext(entry.template || template, { card: entry.card || {}, extraCard: entry.extraCard || {}, uid: entry.uid, set, styling: entry.styling || styling, builtins, spellcheck, cardRegistry: this.registry }));
  }
  initialize() { for (const context of this.contexts) context.initialize(); return this; }
}
