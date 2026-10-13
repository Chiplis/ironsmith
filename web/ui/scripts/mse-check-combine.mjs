import { readFile, writeFile, mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { resolve, join } from 'node:path';
import { execFileSync } from 'node:child_process';
import assert from 'node:assert/strict';
import { combineChannel, imageCombineModes } from '../src/lib/mse/image-combine.mjs';

const [reference] = process.argv.slice(2);
if (!reference) throw new Error('Usage: node scripts/mse-check-combine.mjs NATIVE_MSE_SOURCE_DIRECTORY');
const sourcePath = resolve(reference, 'src/gfx/combine_image.cpp');
const source = await readFile(sourcePath, 'utf8');
const entries = [...source.matchAll(/VALUE_N\("([^"]+)",\s*(COMBINE_\w+)\)/g)].map(m => ({ name: m[1], id: m[2] }));
assert.deepEqual(new Set(entries.map(e => e.name)), new Set(imageCombineModes));
const start = source.indexOf('template <ImageCombine combine> struct Combine');
const end = source.indexOf('// ----------------------------------------------------------------------------- : Combining\r', start);
assert.ok(start >= 0 && end > start, 'Native combining formulas not found');
// Compile the upstream specializations themselves, including their integer
// division. The adapter only supplies enum names and the native clamp helpers.
const cpp = `#include <iostream>
#include <algorithm>
#include <cstdlib>
using namespace std;
enum ImageCombine {${entries.map(e => e.id).join(',')}};
int top(int a){return min(a,255);} int bot(int a){return max(a,0);} int col(int a){return max(0,min(a,255));}
${source.slice(start, end)}
int main(){
${entries.map(e => `for(int a=0;a<256;a++)for(int b=0;b<256;b++)cout.put(static_cast<unsigned char>(Combine<${e.id}>::f(a,b)));`).join('\n')}
}`;
const temp = await mkdtemp(join(tmpdir(), 'ironsmith-mse-combine-'));
try {
  await writeFile(join(temp, 'oracle.cpp'), cpp);
  execFileSync('c++', ['-std=c++17', '-O2', join(temp, 'oracle.cpp'), '-o', join(temp, 'oracle')], { timeout: 30000 });
  const output = execFileSync(join(temp, 'oracle'), [], { timeout: 30000, maxBuffer: 16 * 1024 * 1024 });
  assert.equal(output.length, entries.length * 65536);
  let offset = 0;
  for (const entry of entries) {
    const fn = combineChannel(entry.name);
    for (let a = 0; a < 256; a++) for (let b = 0; b < 256; b++) {
      const expected = output[offset++], actual = fn(a, b);
      if (actual !== expected) assert.fail(`${entry.name}(${a},${b}): expected ${expected}, got ${actual}`);
    }
  }
  console.log(JSON.stringify({ status: 'passed', reference: sourcePath, modes: entries.length, channelPairs: output.length }, null, 2));
} finally { await rm(temp, { recursive: true, force: true }); }
