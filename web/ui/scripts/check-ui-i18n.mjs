import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';
import { localeCatalogs } from '../src/i18n/catalog.js';
const require = createRequire(import.meta.url);
const { parse } = createRequire(require.resolve('eslint'))('espree');
const root = fileURLToPath(new URL('../', import.meta.url));
const known = new Set([...Object.keys(localeCatalogs.en.ui), ...Object.values(localeCatalogs.en.messages)]);
// Brands, units, literal mana syntax and developer commands are language-neutral.
const neutral = new Set(['x', 'ms', 'KB', 'Ironsmith', 'GitHub', '{2}{W}{U}', 'https://...', '__ironsmithDiagnostics.export()']);
const attributes = /^(title|aria-label|ariaLabel|placeholder|alt|label|description|hint|heading|tooltip|text|emptyMessage|[a-zA-Z]+Label)$/;
function files(dir) { return fs.readdirSync(dir, { withFileTypes: true }).flatMap(entry => entry.isDirectory() ? files(path.join(dir, entry.name)) : /\.[jt]sx?$/.test(entry.name) ? [path.join(dir, entry.name)] : []); }
export function auditUiTranslations() {
  const problems = [];
  for (const file of [...files(path.join(root, 'src/components')), path.join(root, 'src/lib/mana-symbols.jsx')]) {
    const source = fs.readFileSync(file, 'utf8');
    const ast = parse(source, { ecmaVersion: 'latest', sourceType: 'module', ecmaFeatures: { jsx: true }, loc: true });
    function report(node, text, kind) { if (/[A-Za-z]/.test(text) && !neutral.has(text)) problems.push(`${path.relative(root, file)}:${node.loc.start.line}: ${kind}: ${JSON.stringify(text)}`); }
    function walk(node) {
      if (!node || typeof node !== 'object') return;
      if (node.type === 'JSXText') report(node, node.value.trim().replace(/\s+/g, ' '), 'unlocalized text');
      if (node.type === 'JSXAttribute' && attributes.test(node.name.name) && node.value?.type === 'Literal') report(node, node.value.value, 'unlocalized attribute');
      if (node.type === 'CallExpression' && ['confirm', 'alert', 'prompt'].includes(node.callee.property?.name) && node.arguments[0]?.type !== 'CallExpression') {
        report(node, node.callee.property.name, 'unlocalized browser dialog');
      }
      if (node.type === 'CallExpression' && node.callee.name === 'ui' && node.arguments[0]?.type === 'Literal') {
        const key = node.arguments[0].value;
        if (typeof key === 'string' && !known.has(key)) report(node, key, 'missing catalog entry');
      }
      for (const [key, value] of Object.entries(node)) if (key !== 'loc') {
        if (Array.isArray(value)) value.forEach(walk); else if (value && typeof value === 'object') walk(value);
      }
    }
    walk(ast);
  }
  return problems;
}
if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const problems = auditUiTranslations();
  console.log(problems.length ? problems.join('\n') : 'UI translation audit passed.');
  process.exitCode = problems.length ? 1 : 0;
}
