import test from 'node:test';
import assert from 'node:assert/strict';
import { MseRuntime } from '../src/lib/mse/script-runtime.mjs';
import { coreBuiltins } from '../src/lib/mse/script-builtins.mjs';

test('choice normalization honors required, exclusive and preferred selections', () => {
  const vm = new MseRuntime({ builtins: coreBuiltins() });
  vm.run(`assert(count_chosen("") == 0)
    assert(count_chosen("red, green", choices:"red,blue") == 1)
    assert(require_choice(choices:"red,green", "red, blue") == "blue, red")
    assert(require_choice(choices:"red,green", "black, blue") == "black, blue, red")
    assert(require_exclusive_choice("red, green", choices:"red,green", last_change:"green") == "green")
    assert(exclusive_choice("red, green, blue", choices:"red,green") == "blue, red")
    assert(remove_choice("red, blue", choice1:"red") == "blue")
    assert(chosen("red, green", choice:"green") == true)
    assert(chosen("red, green", choice:"g") == false)`);
});

test('random collection functions accept injected randomness and enforce counts', () => {
  const vm = new MseRuntime({ builtins: coreBuiltins(), random: () => 0 });
  vm.run(`assert(random_select([123]) == 123)
    assert(sort_list(random_shuffle([1,2,3])) == [1,2,3])
    assert(random_select_many([1], count:3, replace:true) == [1,1,1])
    assert(sort_list(random_select_many([1,2,3], count:3)) == [1,2,3])
    assert(random_int(begin:123, end:124) == 123)
    assert(random_boolean(0.0) == false)
    assert(random_boolean(1.0) == true)`);
  assert.throws(() => vm.run('random_select([])'), { code: 'script-random-empty' });
  assert.throws(() => vm.run('random_select_many([1], count:2)'), { code: 'script-random-count' });
});
