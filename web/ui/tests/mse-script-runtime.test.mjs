import test from 'node:test';
import assert from 'node:assert/strict';
import { MseRuntime } from '../src/lib/mse/script-runtime.mjs';
import { coreBuiltins } from '../src/lib/mse/script-builtins.mjs';
import { MseDelayedError, MseReal, MseFieldValue, MseCollection, code, force, number } from '../src/lib/mse/script-values.mjs';

const runtime = options => new MseRuntime({ builtins: coreBuiltins(), ...options });

test('missing members remain delayed through chained members and calls for or else', () => {
  const vm = runtime();
  assert.equal(vm.run('[].missing() or else "fallback"'), 'fallback');
  assert.equal(vm.run('[].missing.nested or else "fallback"'), 'fallback');
  assert.throws(() => force(vm.run('[].missing()')), { code: 'script-member' });
});

test('field wrappers preserve numeric types, callability and collection iteration', () => {
  const vm = runtime({ variables: { real: new MseFieldValue(new MseReal(1.5), false), values: new MseFieldValue(new MseCollection([2, 3]), true) } });
  assert.equal(number(vm.run('-real')), -1.5);
  assert.equal(vm.run('type_name(real * 2)'), 'double');
  assert.equal(vm.run('type_name(max(real, 2))'), 'double');
  assert.equal(number(vm.run('abs(-real)')), 1.5);
  assert.equal(vm.run('for each n in values do n'), 5);
  vm.scope.set('fn', new MseFieldValue(vm.run('{input + 2}'), true));
  assert.equal(vm.run('fn(3)'), 5);
});

test('MSE dynamic scope, default bindings and recursion differ from lexical JS closures', () => {
  const vm = runtime();
  vm.run(`f := { to_upper(input) }
    h := f@(to_upper:to_lower)
    wrapper := {h()}
    assert(f("aBc", to_upper:to_lower) == "abc")
    assert(h("aBc", to_upper:to_upper) == "ABC")
    assert(wrapper("aBc", to_upper:to_upper) == "abc")
    fib := { if input <= 1 then 1 else fib(input-1) + fib(input-2) }
    assert(fib(6) == 13)
    local := 1
    change := { local := 2; local }
    assert(change() == 2)
    assert(local == 1)`);
  assert.equal(vm.assertions, 6);
});

test('loops sum numbers, concatenate strings and preserve mixed collection iteration', () => {
  const vm = runtime();
  vm.run(`assert((for x from 1 to 6 do x) == 21)
    assert((for x from 1 to 3 do [x]) == [1,2,3])
    assert((for k:v from 2 to 5 do "{k}:{v} ") == "0:2 1:3 2:4 3:5 ")
    assert((for each k:v in [red:"bad", green:"good"] do "{k}={v};") == "green=good;red=bad;")
    assert([[1]].0.0 == 1)
    assert(([a:1] + [a:2]).a == 1)
    assert(length([1, x:2]) == 2)`);
  assert.equal(code(vm.run('["a", y:3, x:2]')), '["a","x":2,"y":3]');
});

test('short circuiting and delayed errors do not use JavaScript truthiness', () => {
  const vm = runtime();
  vm.run(`assert((false and wrong_variable) == false)
    assert((true or wrong_variable) == true)
    assert(("no" and "second") == "no")
    assert(("yes" or "second") == "yes")
    assert((to_int("bad") or else 9) == 9)
    assert((nil or else 9) == nil)
    assert((false or else 9) == false)
    assert(([].absent or else 7) == 7)`);
  assert.throws(() => vm.run('if "anything" then 1 else 2'), { code: 'script-conversion' });
  assert.throws(() => vm.run('unknown or else 2'), { code: 'script-variable' });
  assert.throws(() => vm.run('nil or else unknown'), { code: 'script-variable' });
  assert.ok(vm.run('[].absent') instanceof MseDelayedError);
});

test('real values retain their type and interpolation can return a non-string', () => {
  const vm = runtime();
  vm.run(`assert(type_name(1.0) == "double")
    assert(type_name(1) == "integer")
    assert(type_name(2/1) == "double")
    assert(123.4 mod 5 == 3.4)
    assert(nil != "")
    assert(nil != false)
    assert(nil != 0)
    assert(type_name("{1.0}") == "double")`);
  assert.ok(vm.run('1.0') instanceof MseReal);
  assert.equal(number(vm.run('2^0.5')), Math.sqrt(2));
});

test('rule bindings keep the regex argument separate from the builtin function', () => {
  const vm = runtime();
  vm.run(`f := match_rule(match: "a+|b+")
    assert(f("xyz") == false)
    assert(f("aabb") == true)
    f := match@(match:"a+|b+")
    assert(f("aabb", match:"ss") == false)
    assert(f("ss", match:"ss") == true)`);
});

test('Cajun 2.6 global assignments and zero division behavior', () => {
  const vm = runtime();
  vm.run(`initialize := { shared -> 7 }; initialize()
    increment := { shared := shared + 1 }; increment()
    assert(shared == 8)
    assert(type_name(1 div 0) == "double")`);
  assert.equal(number(vm.run('1 mod 0')), Infinity);
});

test('host data and templates cannot access JavaScript prototypes or host functions', () => {
  const vm = runtime({ variables: { card: { name: 'Example', data: [1, 2] } } });
  assert.equal(vm.run('card.name'), 'Example');
  assert.equal(vm.run('card.data[1]'), 2);
  assert.throws(() => force(vm.run('card.constructor')), { code: 'script-member' });
  assert.throws(() => vm.run('eval("1")'), { code: 'script-variable' });
  assert.throws(() => runtime({ variables: { fn: () => 1 } }), { code: 'script-host-value' });
});

test('long init scripts avoid sequence stack overflow; runaway loops and recursion are bounded', () => {
  const vm = runtime();
  assert.equal(vm.run(Array.from({ length: 12000 }, (_, i) => `x := ${i}`).join('\n')), 11999);
  assert.throws(() => runtime({ maxSteps: 100 }).run('for x from 1 to 1000 do x'), { code: 'script-budget' });
  const recursive = runtime({ maxDepth: 20 });
  assert.throws(() => recursive.run('f := { f() }; f()'), { code: 'script-depth' });
  assert.equal(recursive.depth, 0);
  assert.equal(recursive.run('2+2'), 4);
});

test('string members index Unicode characters and keep invalid lookups delayed', () => {
  const vm = runtime();
  vm.run(`assert("WUBRG".0 == "W")
    assert(reverse("WUBRG").0 == "G")
    assert("A😀é".1 == "😀")
    assert("A😀é".2 == "é")
    assert(("".0 or else "none") == "none")
    assert(("a".2 or else "none") == "none")
    assert(("abc".constructor or else "none") == "none")`);
  vm.scope.set('field', new MseFieldValue('UB', false));
  assert.equal(vm.run('field.1'), 'B');
});

test('host evaluations have independent budgets while nested callbacks share the caller budget',()=>{
  const vm=runtime({maxSteps:60});
  const fn=vm.run('{for x from 1 to 8 do x}');
  for(let i=0;i<30;i++){
    assert.equal(vm.evaluate(fn.body,vm.scope),36);
    assert.equal(vm.invoke(fn,[],vm.scope),36);
  }
  const nested=runtime({maxSteps:60,builtins:{...coreBuiltins(),burst:(_scope,r)=>r.run('for x from 1 to 8 do x')}});
  assert.throws(()=>nested.run('for y from 1 to 20 do burst()'),{code:'script-budget'});
  assert.equal(nested.executionDepth,0);
  assert.equal(nested.depth,0);
  assert.equal(nested.run('2+2'),4);
});
