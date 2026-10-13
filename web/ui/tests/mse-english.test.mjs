import test from 'node:test';
import assert from 'node:assert/strict';
import {processEnglishHints as hints} from '../src/lib/mse/english-hints.mjs';
import {MseRuntime} from '../src/lib/mse/script-runtime.mjs';
import {coreBuiltins} from '../src/lib/mse/script-builtins.mjs';

test('English hints consume singular/plural suffixes once and preserve unrelated markup',()=>{
  assert.equal(hints('<hint-1>one card(s) (again)'), 'one card (again)');
  assert.equal(hints('<hint-2>two card(s) (again)'), 'two cards (again)');
  assert.equal(hints('<hint-x><b>cards</b> (again)'), '<b>cards</b> (again)');
  assert.equal(hints('<hint-1><singular>is</singular><plural>are</plural> ready'), 'is ready');
  assert.equal(hints('<hint-2><singular>is</singular><plural>are</plural> ready'), 'are ready');
  assert.equal(hints('<hint-2><plural><b>are</b></plural><singular>is</singular>'), '<b>are</b>');
});
test('English hints handle tagged articles and empty numeric parameters',()=>{
  assert.equal(hints('A <param-type>Elf</param-type>'), 'An <param-type>Elf</param-type>');
  assert.equal(hints('a <param-type>Goblin</param-type>'), 'a <param-type>Goblin</param-type>');
  assert.equal(hints('mana <param-type>Elf</param-type>'), 'mana <param-type>Elf</param-type>');
  assert.equal(hints('<hint-1><param-number>a</param-number> artifact(s)'), '<param-number>an</param-number> artifact');
  assert.equal(hints('Draw <param-number></param-number> card.'), 'Draw<param-number></param-number> card.');
});
test('English hints reject unmatched alternatives and execute through the script builtin',()=>{
  for(const value of ['<singular>is','<singular>is</singular>','<plural>are</plural>'])assert.throws(()=>hints(value),{code:'script-english'});
  const vm=new MseRuntime({builtins:coreBuiltins()});
  assert.equal(vm.run('process_english_hints("<hint-2>card(s)")'),'cards');
});

test('English numeric builtins preserve native cardinal, ordinal and one-specific forms',()=>{
  const vm=new MseRuntime({builtins:coreBuiltins()});
  for(const [input,expected,ordinal] of [[0,'zero','0th'],[1,'one','first'],[12,'twelve','twelfth'],[14,'fourteen','fourteenth'],[18,'eighteen','eighteenth'],[20,'twenty','twentieth'],[23,'twenty-three','twenty-third'],[60,'sixty','sixtieth'],[99,'ninety-nine','ninety-ninth'],[101,'101','101th'],[-1,'-1','-1th']]){
    assert.equal(vm.run(`english_number(${input})`),expected);
    assert.equal(vm.run(`english_number_ordinal(${input})`),ordinal);
  }
  assert.equal(vm.run('english_number_a(1)'),'a');
  assert.equal(vm.run('english_number_multiple(1)'),'');
  assert.equal(vm.run('english_number("1.5")'),'1.5');
  assert.equal(vm.run('english_number("X")'),'X');
});
test('English numeric parameters retain markup and drive later singular/plural hints',()=>{
  const vm=new MseRuntime({builtins:coreBuiltins()});
  assert.equal(vm.run('english_number("<param-number>23</param-number>")'),'<hint-2><param-number>twenty-three</param-number>');
  assert.equal(vm.run('english_number("<param-number>X</param-number>")'),'<hint-2><param-number>X</param-number>');
  assert.equal(vm.run('process_english_hints(english_number_a("<param-number>1</param-number>") + " artifact(s)")'),'<param-number>an</param-number> artifact');
  assert.equal(vm.run('process_english_hints(english_number("<param-number>2</param-number>") + " card(s)")'),'<param-number>two</param-number> cards');
});
