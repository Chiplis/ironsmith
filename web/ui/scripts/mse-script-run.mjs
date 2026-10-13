import { readFile } from 'node:fs/promises';
import { resolve, dirname, basename } from 'node:path';
import { execFileSync } from 'node:child_process';
import { MsePackageStore } from '../src/lib/mse/packages.mjs';
import { MseScriptLoader } from '../src/lib/mse/script-loader.mjs';
import { MseRuntime } from '../src/lib/mse/script-runtime.mjs';
import { coreBuiltins } from '../src/lib/mse/script-builtins.mjs';
import { code, force } from '../src/lib/mse/script-values.mjs';

const [filename, dictionaryRoot] = process.argv.slice(2);
if (!filename) throw new Error('Usage: node scripts/mse-script-run.mjs SCRIPT_FILE [HUNSPELL_DICTIONARY_DIRECTORY]');
const file = resolve(filename);
const store = new MsePackageStore({ readText: path => readFile(resolve(dirname(dirname(file)), path.key), 'utf8') });
const loader = new MseScriptLoader(store);
const spellcheck = dictionaryRoot ? (word, language) => {
  if (!/^[a-zA-Z0-9_-]+$/.test(language)) throw new Error('Invalid dictionary name');
  return execFileSync('hunspell', ['-d', resolve(dictionaryRoot, language), '-l'], { input: word + '\n', encoding: 'utf8', timeout: 10000 }).trim() === '';
} : undefined;
const runtime = new MseRuntime({ builtins: coreBuiltins({ spellcheck }) });
const providers = { spelling: dictionaryRoot ? { implementation: 'native-hunspell', dictionaryRoot: resolve(dictionaryRoot) } : null };
try {
  const ast = await loader.compile(await readFile(file, 'utf8'), { file, package: basename(dirname(file)) });
  const result = force(runtime.run(ast));
  console.log(JSON.stringify({ file, providers, status: 'passed', assertions: runtime.assertions, steps: runtime.steps, result: code(result) }, null, 2));
} catch (error) {
  console.log(JSON.stringify({ file, providers, status: 'failed', assertions: runtime.assertions, steps: runtime.steps, error: { code: error.code || 'host', message: error.message } }, null, 2));
  process.exitCode = 1;
}
