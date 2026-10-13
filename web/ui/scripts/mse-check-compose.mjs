import { readFile, writeFile, mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { resolve, join } from 'node:path';
import { execFileSync } from 'node:child_process';
import assert from 'node:assert/strict';
import { composePixel } from '../src/lib/mse/image-insert.mjs';

const [reference] = process.argv.slice(2);
if (!reference) throw new Error('Usage: node scripts/mse-check-compose.mjs WXWIDGETS_IMAGE_CPP');
const sourcePath = resolve(reference), source = await readFile(sourcePath, 'utf8');
const compose = source.indexOf('case wxIMAGE_ALPHA_BLEND_COMPOSE:');
const loop = source.indexOf('for (int i = 0; i < width; i++)', compose);
const start = source.indexOf('{', loop);
assert.ok(compose >= 0 && loop > compose && start > loop);
let end = start + 1, depth = 1;
while (depth && end < source.length) { if (source[end] === '{') depth++; if (source[end] === '}') depth--; end++; }
assert.equal(depth, 0);
const fixtures = [[0,255,128,255,0,127], [1,63,254,254,192,1], [101,151,201,49,99,149], [127,128,129,129,128,127]];
const cpp = `#include <iostream>
int main(){
int colors[][6]={${fixtures.map(f => `{${f}}`).join(',')}};
for(auto& colorset:colors)for(int ta=0;ta<256;ta++)for(int sa=0;sa<256;sa++){
unsigned char target_data[3],source_data[3],alpha_target_data[1]={(unsigned char)ta},alpha_source_data[1]={(unsigned char)sa};
for(int c=0;c<3;c++){target_data[c]=colorset[c];source_data[c]=colorset[c+3];}
int width=1;
${source.slice(loop, end)}
for(int c=0;c<3;c++)std::cout.put(target_data[c]);std::cout.put(alpha_target_data[0]);
}}
`;
const temp = await mkdtemp(join(tmpdir(), 'ironsmith-mse-compose-'));
try {
  await writeFile(join(temp, 'oracle.cpp'), cpp);
  execFileSync('c++', ['-std=c++17', '-O2', '-ffp-contract=off', join(temp, 'oracle.cpp'), '-o', join(temp, 'oracle')], { timeout: 30000 });
  const output = execFileSync(join(temp, 'oracle'), [], { timeout: 30000, maxBuffer: 2 * 1024 * 1024 });
  assert.equal(output.length, fixtures.length * 65536 * 4);
  let offset = 0;
  for (const colors of fixtures) for (let ta = 0; ta < 256; ta++) for (let sa = 0; sa < 256; sa++) {
    const target = new Uint8ClampedArray([...colors.slice(0, 3), ta]);
    const source = new Uint8ClampedArray([...colors.slice(3), sa]);
    composePixel(target, 0, source, 0, true);
    for (let c = 0; c < 4; c++) {
      const expected = output[offset++];
      if (target[c] !== expected) assert.fail(`Alpha ${ta},${sa}, colors ${colors}, channel ${c}: expected ${expected}, got ${target[c]}`);
    }
  }
  console.log(JSON.stringify({ status: 'passed', reference: sourcePath, pixels: output.length / 4, comparedBytes: output.length }, null, 2));
} finally { await rm(temp, { recursive: true, force: true }); }
