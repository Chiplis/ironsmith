import test from 'node:test';
import assert from 'node:assert/strict';
import { MseRuntime } from '../src/lib/mse/script-runtime.mjs';
import { coreBuiltins } from '../src/lib/mse/script-builtins.mjs';

const marked = word => `<error-spelling:en_US>${word}</error-spelling:en_US>`;
function vm(options = {}) { return new MseRuntime({ builtins: coreBuiltins(options) }); }

test('spelling checks words across formatting and preserves punctuation and Unicode', () => {
  const words = [];
  const runtime = vm({ spellcheck: word => { words.push(word); return word === 'hello'; } });
  runtime.scope.set('source', 'hello, wr<i>ng</i>! café isn’t 123');
  assert.equal(runtime.run('check_spelling(source, language:"en_US")'),
    `hello, ${marked('wr<i>ng</i>')}! ${marked('café')} ${marked('isn’t')} 123`);
  assert.deepEqual(words, ['hello', 'wrng', 'café', 'isn’t']);
});

test('nested protected tags suppress checking, but similar tag names do not', () => {
  const words = [];
  const runtime = vm({ spellcheck: word => { words.push(word); return false; } });
  const source = '<nospellcheck>a <sym>b</sym> c</nospellcheck> <atom-name>d</atom-name> <symbol>e</symbol>';
  runtime.scope.set('source', source);
  assert.equal(runtime.run('check_spelling(source, language:"en_US")'),
    source.replace('e</symbol>', marked('e</symbol>')));
  assert.deepEqual(words, ['e']);
});

test('old marks are removed with no language; disabled checking preserves input', () => {
  const source = marked('bad');
  for (const [options, expected] of [[{}, 'bad'], [{ spellcheckEnabled: false }, source]]) {
    const runtime = vm(options);
    runtime.scope.set('source', source);
    assert.equal(runtime.run('check_spelling(source, language:"")'), expected);
  }
  assert.throws(() => vm().run('check_spelling("bad", language:"en_US")'), { code: 'script-spelling-provider' });
  assert.throws(() => vm({ spellcheck: () => Promise.resolve(true) }).run('check_spelling_word("bad", language:"en_US")'), { code: 'script-spelling-provider' });
});

test('supplemental dictionaries and exceptions are consulted after language dictionary', () => {
  const calls = [];
  const runtime = vm({ spellcheck: (word, language, dictionary) => {
    calls.push([word, language, dictionary]);
    return word === 'wizard' && dictionary === '/magic/words';
  } });
  assert.equal(runtime.run('check_spelling("wizard custom wr<i>ng</i>", language:"en_US", extra_dictionary:"/magic/words", extra_match:{input == "custom" or input == "wr<i>ng</i>"})'), 'wizard custom wr<i>ng</i>');
  assert.deepEqual(calls, ['wizard', 'custom', 'wrng'].flatMap(word => [[word, 'en_US', undefined], [word, 'en_US', '/magic/words']]));
});
