import { readFile, readdir, writeFile } from 'node:fs/promises';
import { resolve, join } from 'node:path';
import { MsePackageStore } from '../src/lib/mse/packages.mjs';
import { MseScriptLoader } from '../src/lib/mse/script-loader.mjs';

const [directory, output] = process.argv.slice(2);
if (!directory) throw new Error('Usage: node scripts/mse-audit.mjs CORPUS_DIRECTORY [REPORT.json]');
const root = resolve(directory);
const source = JSON.parse(await readFile(join(root, 'source.json'), 'utf8'));
const store = new MsePackageStore({
  readText: path => readFile(join(root, 'data', path.key), 'utf8'),
  assetUrl: path => join(root, 'data', path.key),
});
const scripts = new MseScriptLoader(store);
const names = (await readdir(join(root, 'data'))).filter(name => /\.mse-(style|game|include|symbol-font|export-template|locale)$/.test(name)).sort();
const results = [];
for (const name of names) {
  const result = { name, status: 'metadata-loaded' };
  try {
    const pkg = await store.package(name);
    result.version = pkg.version;
    result.mseVersion = pkg.mseVersion;
    result.dependencies = pkg.dependencies;
    const doc = await store.expandRecord(pkg.document);
    result.keys = [...new Set(doc.entries.map(entry => entry.key))];
    result.scripts = [];
    for (const entry of doc.all('init script')) {
      const script = { file: entry.location.file, line: entry.location.line, characters: entry.text().length };
      try {
        await scripts.compile(entry.text(), { ...entry.location, line: entry.location.line + (entry.inline ? 0 : 1) });
        script.status = 'parsed';
      } catch (error) {
        script.status = 'parse-error';
        script.error = { code: error.code || 'io', message: error.message };
      }
      result.scripts.push(script);
    }
    if (name.endsWith('.mse-style')) {
      result.dimensions = [doc.text('card width'), doc.text('card height')];
      result.fields = [];
      for (const section of doc.all('card style')) {
        const fields = await store.expandRecord(section.record());
        result.fields.push(...fields.entries.map(entry => entry.key));
      }
    }
    try {
      result.dependencyClosure = (await store.dependencyClosure(name)).map(pkg => pkg.name);
    } catch (error) {
      result.status = 'dependency-error';
      result.error = { code: error.code || 'io', message: error.message };
    }
  } catch (error) {
    result.status = 'read-error';
    result.error = { code: error.code || 'io', message: error.message };
  }
  results.push(result);
}
const counts = Object.fromEntries([...new Set(results.map(r => r.status))].map(status => [status, results.filter(r => r.status === status).length]));
const report = {
  source, counts, packages: results,
  // Reading metadata does not establish runtime/rendering compatibility.
  completion: { scriptExecution: false, imageComposition: false, textLayout: false, interactiveRenderer: false, visualParity: false },
};
const scriptFailures = results.flatMap(pkg => (pkg.scripts || []).filter(script => script.status !== 'parsed').map(script => ({ package: pkg.name, ...script })));
report.scriptCounts = { parsed: results.flatMap(pkg => pkg.scripts || []).length - scriptFailures.length, failed: scriptFailures.length };
if (output) await writeFile(resolve(output), JSON.stringify(report, null, 2) + '\n');
console.log(JSON.stringify({ source, counts, scriptCounts: report.scriptCounts, scriptFailures: scriptFailures.slice(0, 12), failures: results.filter(r => r.status !== 'metadata-loaded').slice(0, 12).map(({ name, error }) => ({ name, error })) }, null, 2));
