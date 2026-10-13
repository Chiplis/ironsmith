import test from 'node:test';
import assert from 'node:assert/strict';
import { MseRuntime } from '../src/lib/mse/script-runtime.mjs';
import { coreBuiltins } from '../src/lib/mse/script-builtins.mjs';

test('collection sorting uses evaluated keys, natural order and duplicate keys', () => {
  const vm = new MseRuntime({ builtins: coreBuiltins() });
  vm.run(`assert(sort_list([5,2,3,1,4]) == [1,2,3,4,5])
    assert(sort_list(["aaa","cccc","bb"], order_by:length) == ["bb","aaa","cccc"])
    assert(sort_list(["c10","C2","c1"]) == ["c1","C2","c10"])
    assert(sort_list([1,2,1,2,3], remove_duplicates:true) == [1,2,3])
    assert(sort_list(["bb","cc","aaa"], order_by:length, remove_duplicates:true) == ["bb","aaa"])
    assert(position(of:3, in:[1,2,3]) == 2)
    assert(number_of_items(in:[a:1,b:2]) == 2)`);
  assert.equal(vm.assertions, 7);
});

test('MSE color conversions preserve alpha and explicit conversion failures', () => {
  const vm = new MseRuntime({ builtins: coreBuiltins() });
  vm.run(`assert(to_string(to_color("blue")) == "rgb(0,0,255)")
    assert(to_color("rgb(0,0,0,200)") == rgb(0,0,0))
    assert(to_color("rgb(255,255,255,200)") == rgb(255,255,255))
    assert(to_color("rgba(1,2,3,200)") == rgba(1,2,3,200))
    assert(to_color(nil) == rgba(0,0,0,0))
    assert(to_color("#f008") == rgba(255,0,0,136))
    assert(to_color("#12345678") == rgba(18,52,86,120))
    assert(to_int(rgb(255,255,255)) == 255)
    assert((to_color("unknown") or else "failed") == "failed")`);
  assert.equal(vm.assertions, 9);
});
