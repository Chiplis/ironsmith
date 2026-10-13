import { corpusStore } from './mse-corpus-store.mjs';
import { loadMseTemplate, MseTemplateContext } from '../src/lib/mse/template.mjs';

const [directory, style = 'magic-m15-altered.mse-style'] = process.argv.slice(2);
if (!directory) throw new Error('Usage: node scripts/mse-template-probe.mjs CORPUS_DIRECTORY [STYLE]');
const store = await corpusStore(directory);
try {
  const template = await loadMseTemplate(store, style);
  console.log(JSON.stringify({ style, status: 'loaded', fields: Object.fromEntries(Object.entries(template.definitions).map(([key, fields]) => [key, fields.size])) }));
  const context = new MseTemplateContext(template).initialize();
  console.log(JSON.stringify({ style, status: 'initialized', variables: context.runtime.scope.values.size }));
  const layout = context.layout();
  console.log(JSON.stringify({ style, status: 'layout-evaluated', dimensions: [layout.width, layout.height], fields: layout.fields.length }));
} catch (error) {
  console.log(JSON.stringify({ style, status: 'failed', error: { code: error.code || 'host', message: error.message } }, null, 2));
  process.exitCode = 1;
}
