import test from 'node:test';
import assert from 'node:assert/strict';
import {MseRuntime} from '../src/lib/mse/script-runtime.mjs';
import {coreBuiltins} from '../src/lib/mse/script-builtins.mjs';
import {formatCases} from './fixtures/mse-format-cases.mjs';

test('numeric and string format matches pinned native padding, bases, precision and rounding',()=>{
  const runtime=new MseRuntime({builtins:coreBuiltins()});
  for(const {expression,expected} of formatCases)assert.equal(runtime.run(expression),expected,expression);
});
test('to_string shares native format behavior while preserving delayed errors',()=>{
  const runtime=new MseRuntime({builtins:coreBuiltins()});
  for(const {expression,expected} of formatCases){
    const converted=expression.replace(/^format\(/,'to_string(');
    assert.equal(runtime.run(converted),expected,converted);
  }
  assert.equal(runtime.run('to_string(7,format:"Q") or else "fallback"'),'fallback');
  assert.throws(()=>runtime.run('format(7,format:"Q")'),{code:'script-format'});
});
test('format output budgets reject excessive padding before allocation',()=>{
  const runtime=new MseRuntime({builtins:coreBuiltins()});
  assert.throws(()=>runtime.run('format(7,format:"1000001d")'),{code:'script-format'});
  assert.throws(()=>runtime.run('format(1.0,format:".10001f")'),{code:'script-format'});
});
