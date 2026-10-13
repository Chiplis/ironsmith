import test from 'node:test';
import assert from 'node:assert/strict';
import { MseRuntime } from '../src/lib/mse/script-runtime.mjs';
import { coreBuiltins } from '../src/lib/mse/script-builtins.mjs';
import { compileMseRegex } from '../src/lib/mse/regex.mjs';

test('Boost extended regex syntax preserves class whitespace, hashes and escaped literals', () => {
  const re = compileMseRegex(String.raw`(?ix)
    ^ [[:space:]"(] # boundary
    ( [A-Z]+ ) [ ] \# \  (?# a regex comment)
    [[:digit:]]+ $`);
  assert.equal(re.test(' abc # 123'), true);
  assert.equal(re.test('(ABC # 123'), true);
  assert.equal(re.test('xABC # 123'), false);
  assert.equal(compileMseRegex('(?x)^[ #]+$').test(' # '), true);
  assert.equal(compileMseRegex('(?x)^a(?-x: b )c$').test('a b c'), true);
  assert.equal(compileMseRegex('^a(?x: b # comment\n c)d$').test('abcd'), true);
  assert.equal(compileMseRegex('(?i)(?-i)^a$').test('A'), false);
  assert.equal(compileMseRegex('(?-s)^a.b$').test('a\nb'), false);
  assert.equal(compileMseRegex('^b$').test('a\nb\nc'), true);
  assert.equal(compileMseRegex('(?-m)^b$').test('a\nb\nc'), false);
});

test('Boost anchors, quoted text and POSIX classes translate without changing capture numbers', () => {
  assert.deepEqual(compileMseRegex('([[:alpha:]_]+)([[:digit:]]+)').exec('name_123').slice(1), ['name_', '123']);
  assert.equal(compileMseRegex('^[[:punct:]]+$').test('[]!{}#'), true);
  assert.equal(compileMseRegex('^[^[:space:]]+$').test('a b'), false);
  assert.equal(compileMseRegex(String.raw`(?m)\Aa\z`).test('a\na'), false);
  assert.equal(compileMseRegex(String.raw`a\Z`).test('a\n'), true);
  assert.equal(compileMseRegex(String.raw`a\z`).test('a\n'), false);
  assert.equal(compileMseRegex(String.raw`\<cat\>`).test('catfish cat'), true);
  assert.equal(compileMseRegex(String.raw`\<cat\>`).test('catfish'), false);
  assert.equal(compileMseRegex(String.raw`(?x)^\Qa # b.*\E$`).test('a # b.*'), true);
  assert.equal(compileMseRegex(String.raw`^a\Rb$`).test('a\r\nb'), true);
  assert.equal(compileMseRegex('^[]a]+$').test(']a'), true);
  assert.equal(compileMseRegex('[^^]').test('a'), true);
  assert.equal(compileMseRegex('[^^]').test('^'), false);
  assert.equal(compileMseRegex('[[.quotation-mark.]]|”').test('"'), true);
  assert.equal(compileMseRegex('^[[.left-square-bracket.]][[.A.]-[.Z.]][[.right-square-bracket.]]$').test('[B]'), true);
  assert.throws(() => compileMseRegex('[[:unknown:]]'), { code: 'script-regex' });
  assert.throws(() => compileMseRegex(String.raw`a\Kb`), { code: 'script-regex' });
});

test('MSE regex rules support contexts, split fields, captures and sed replacement strings', () => {
  const vm = new MseRuntime({ builtins: coreBuiltins() });
  vm.run(`assert(break_text("a,b,c", match:"[^,]+") == ["a","b","c"])
    assert(break_rule(match:"[^,]+")("a,b,c") == ["a","b","c"])
    assert(split_text("a,b,,c,", match:",") == ["a","b","","c",""])
    assert(split_text("a,b,,c,", match:",", include_empty:false) == ["a","b","c"])
    assert(split_text("", match:",") == [""])
    assert(filter_text(match:".", in_context:"a<match>", "banana") == "nn")
    assert(replace(match:"[ab]", replace:to_upper, "banana") == "BAnAnA")
    assert(replace(match:"([0-9])[*]([0-9])", replace:{_1*_2}, "2*2+3*3") == "4+9")
    assert(replace(match:" ", replace:"[&|&]", "a b c") == "a[ | ]b[ | ]c")
    assert(replace(match:" ", replace:"x", "a b c d", in_context:"<match>[cd]") == "a bxcxd")
    assert(match("ABC", match:"(?i)abc") == true)`);
  assert.equal(vm.assertions, 11);
  assert.throws(() => vm.run('break_text("x", match:"")'), { code: 'script-regex-empty' });
});
