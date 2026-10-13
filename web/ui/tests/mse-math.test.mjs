import test from 'node:test';
import assert from 'node:assert/strict';
import { MseRuntime } from '../src/lib/mse/script-runtime.mjs';
import { coreBuiltins } from '../src/lib/mse/script-builtins.mjs';
import { MseReal, number } from '../src/lib/mse/script-values.mjs';

test('native math functions preserve real results and radian/degree conventions', () => {
  const vm = new MseRuntime({ builtins: coreBuiltins() });
  for (const [script, expected] of [
    ['sin(0)', 0], ['cos(0)', 1], ['tan(0)', 0],
    ['sin_deg(90)', 1], ['cos_deg(180)', -1], ['tan_deg(45)', 1],
    ['exp(0)', 1], ['log(1)', 0], ['log10(100)', 2],
    ['sqrt(9)', 3], ['pow(2, exponent:-3)', 0.125],
  ]) {
    const actual = vm.run(script);
    assert.ok(actual instanceof MseReal, script);
    assert.ok(Math.abs(number(actual) - expected) < 1e-12, script);
  }
  assert.equal(number(vm.run('log(0)')), -Infinity);
  assert.ok(Number.isNaN(number(vm.run('sqrt(-1)'))));
  assert.equal(vm.run('reverse("WUBRG")'), 'GRBUW');
  assert.equal(vm.run('reverse("A😀é")'), 'é😀A');
});
