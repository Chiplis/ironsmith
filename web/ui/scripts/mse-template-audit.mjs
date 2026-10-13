import { readdir, mkdir, writeFile } from 'node:fs/promises';
import { resolve, dirname } from 'node:path';
import { corpusStore } from './mse-corpus-store.mjs';
import { loadMseTemplate, MseTemplateContext } from '../src/lib/mse/template.mjs';
import upstream from '../src/lib/mse/upstream.mjs';

const [directory, output = 'reports/mse/template-runtime.json'] = process.argv.slice(2);
if (!directory) throw new Error('Usage: node scripts/mse-template-audit.mjs CORPUS_DIRECTORY [REPORT.json]');
const root = resolve(directory, 'data');
const styles = (await readdir(root, { withFileTypes: true })).filter(entry => entry.isDirectory() && entry.name.endsWith('.mse-style')).map(entry => entry.name).sort();
const store = await corpusStore(directory);
const results = [];
for (const style of styles) {
  let phase = 'load';
  const result = { style, loaded: false, initialized: false, layoutEvaluated: false };
  try {
    const template = await loadMseTemplate(store, style);
    result.loaded = true;
    result.keywords = template.keywords.length;
    phase = 'initialize';
    const context = new MseTemplateContext(template).initialize();
    result.initialized = true;
    phase = 'layout';
    const layout = context.layout();
    result.layoutEvaluated = true;
    result.dimensions = [layout.width, layout.height];
    result.fields = layout.fields.length;
  } catch (error) {
    result.failure = { phase, code: error.code || 'host', message: error.message };
  }
  results.push(result);
  if (results.length % 20 === 0) console.error(`${results.length}/${styles.length} templates checked`);
}
const report = {
  upstream, generatedAt: new Date().toISOString(),
  scope: 'Default card, set and styling values; no image drawing or visual comparison. Successful layout evaluation does not establish rendering parity.',
  summary: {
    styles: results.length,
    loaded: results.filter(result => result.loaded).length,
    initialized: results.filter(result => result.initialized).length,
    layoutEvaluated: results.filter(result => result.layoutEvaluated).length,
    renderingVerified: false,
  },
  results,
};
await mkdir(dirname(resolve(output)), { recursive: true });
await writeFile(output, JSON.stringify(report, null, 2) + '\n');
console.log(JSON.stringify({ ...report.summary, report: resolve(output) }, null, 2));
if (results.some(result => result.failure)) process.exitCode = 1;
