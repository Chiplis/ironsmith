import test from 'node:test';
import assert from 'node:assert/strict';
import { MseRuntime } from '../src/lib/mse/script-runtime.mjs';
import { coreBuiltins } from '../src/lib/mse/script-builtins.mjs';

test('tag content replacement preserves tags, nested content and dynamic arguments', () => {
  const vm = new MseRuntime({ builtins: coreBuiltins() });
  vm.run(`assert(tag_contents(tag:"<up>", contents:to_upper, "this <up>is text</up>") == "this <up>IS TEXT</up>")
    assert(tag_contents(tag:"<atom-name", contents:{card_name}, "<atom-name-auto></atom-name-auto> loses 1 life", card_name:"Pink Elephant") == "<atom-name-auto>Pink Elephant</atom-name-auto> loses 1 life")
    assert(tag_contents(tag:"<x>", contents:{input + "!"}, "<x>a<x>b</x>c</x>") == "<x>a<x>b</x>c!</x>")
    assert(remove_tag("<b>Bold <i>italic</i></b>", tag:"<b>") == "Bold <i>italic</i>")
    assert(remove_tags("<b>bold</b> text") == "bold text")
    assert(tag_remove_rule(tag:"<b>")("<b>bold</b>") == "bold")
    assert(tag_contents_rule(tag:"<up>", contents:to_upper)("<up>text</up>") == "<up>TEXT</up>")`);
  assert.equal(vm.run('curly_quotes("\\\"hello\\\", —\\\"world\\\"")'), '“hello”, —“world”');
  assert.equal(vm.run('curly_quotes("\'\'\\\"nest\\\"\'\'")'), '‘‘“nest”’’');
  assert.throws(() => vm.run('check_spelling_word("word", language:"en_US")'), { code: 'script-spelling-provider' });
});
