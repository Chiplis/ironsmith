import test from 'node:test';
import assert from 'node:assert/strict';
import { parseMseScript as parse } from '../src/lib/mse/script-parser.mjs';

test('MSE arithmetic and boolean precedence is preserved', () => {
  const power = parse('3^3^3');
  assert.equal(power.right.operator, '^');
  assert.equal(parse('-2^2').left.type, 'unary');
  const logic = parse('true or false and true');
  assert.equal(logic.operator, 'and');
  assert.equal(logic.left.operator, 'or');
  assert.equal(parse('1 + 2\n * 3').right.operator, '*');
});

test('functions, named arguments, binding, optional commas and newline sequences', () => {
  const ast = parse('f := { if input then card.name else "" }\ng := f@(input:true)\ng()');
  assert.equal(ast.type, 'sequence');
  assert.equal(ast.right.type, 'call');
  assert.equal(ast.left.right.right.type, 'bind');
  const call = parse('blend(a:1 b:2,)');
  assert.deepEqual(call.arguments.map(a => a.name), ['a', 'b']);
  assert.equal(parse('rgb(1,2,3)').arguments.length, 3);
  assert.equal(parse('x := y := 3').right.type, 'assign');
});

test('collections support mixed keys, dot-number members, loops and cases', () => {
  const collection = parse('["w", name: "white", 3: true,]');
  assert.equal(collection.entries[0].key, null);
  assert.equal(collection.entries[2].key.value, '3');
  assert.equal(parse('[[1]].0.0').key.value, '0');
  const loop = parse('for each k:v in [a:1] do "{k}:{v}"');
  assert.equal(loop.key, 'k');
  assert.equal(loop.name, 'v');
  assert.equal(loop.body.type, 'interpolate');
  assert.equal(parse('for n from 1 to 3 do [n]').end.value, 3);
  assert.equal(parse('case x of 1: "one", 2: "two" else: "other"').cases.length, 2);
});

test('interpolation nests scripts, function braces and quoted strings', () => {
  const ast = parse('"a{if true then { "nested {card.name}" }() else ""}z"');
  assert.equal(ast.parts[1].type, 'if');
  assert.equal(ast.parts[1].yes.callee.body.parts[1].type, 'member');
  assert.equal(parse('plain {card.name}', { stringMode: true }).parts[1].type, 'member');
  assert.equal(parse('"\\{W\\}\\n\\&"').value, '{W}\n');
});

test('script includes retain ownership and diagnostics retain source line numbers', () => {
  const ast = parse('# comment\ninclude file: /magic.mse-game/script\nx := 2', { file: 'style', package: 'a.mse-style', line: 8 });
  assert.equal(ast.left.type, 'include');
  assert.equal(ast.left.location.line, 9);
  assert.equal(ast.left.location.package, 'a.mse-style');
  assert.equal(parse('include_file("script")').path, 'script');
  assert.throws(() => parse('card.name := 2'), { code: 'script-syntax' });
  assert.throws(() => parse('"unclosed {x"'), { code: 'script-syntax' });
  assert.throws(() => parse('if true 2'), { code: 'script-syntax' });
});
