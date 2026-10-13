import test from 'node:test';
import assert from 'node:assert/strict';
import {MseFieldUpdateQueue} from '../src/lib/mse/field-update-queue.mjs';

test('initialization reads stored values in order and cyclic updates run once per transaction',()=>{
  const queue=new MseFieldUpdateQueue(),values={a:0,b:0},calls=[];
  const read=key=>{queue.read(key);return values[key];};
  queue.register('a',()=>{calls.push('a');const next=read('b')+1,changed=next!==values.a;values.a=next;return changed;});
  queue.register('b',()=>{calls.push('b');const next=read('a')+1,changed=next!==values.b;values.b=next;return changed;});
  queue.initialize();assert.deepEqual(values,{a:1,b:2});assert.deepEqual(calls,['a','b']);
  calls.length=0;queue.change('a');
  assert.deepEqual(values,{a:3,b:4});assert.deepEqual(calls,['a','b']);
});
test('FIFO propagation deduplicates diamond dependencies and refreshes conditional reads',()=>{
  const queue=new MseFieldUpdateQueue(),values={input:0,other:9,branch:false,a:0,b:0,end:0},calls=[];
  const read=key=>{queue.read(key);return values[key];};
  for(const key of ['input','other','branch'])queue.register(key,()=>false);
  for(const key of ['a','b'])queue.register(key,()=>{calls.push(key);const next=read('branch')?read('other'):read('input'),changed=values[key]!==next;values[key]=next;return changed;});
  queue.register('end',()=>{calls.push('end');const next=read('a')+read('b'),changed=values.end!==next;values.end=next;return changed;});
  queue.initialize();calls.length=0;
  values.input=3;queue.change('input');assert.deepEqual(calls,['a','b','end']);assert.equal(values.end,6);
  values.branch=true;queue.change('branch');assert.equal(values.end,18);
  calls.length=0;values.input=4;queue.change('input');assert.deepEqual(calls,[]);
});
test('a failed update leaves the queue reusable and retains previous dependencies',()=>{
  const queue=new MseFieldUpdateQueue();let fail=false,calls=0;
  queue.register('source',()=>false);
  queue.register('dependent',()=>{queue.read('source');if(fail)throw new Error('failure');calls++;return true;});
  queue.initialize();fail=true;assert.throws(()=>queue.change('source'),/failure/);
  assert.equal(queue.running,false);assert.equal(queue.active,null);
  fail=false;queue.change('source');assert.equal(calls,2);
});

test('simultaneous value and metadata changes notify a shared dependent once',()=>{
  const queue=new MseFieldUpdateQueue();let calls=0;
  queue.register('value',()=>false);queue.register('timestamp',()=>false);
  queue.register('dependent',()=>{queue.read('value');queue.read('timestamp');calls++;return true;});
  queue.initialize();calls=0;
  queue.changeMany(['value','timestamp']);assert.equal(calls,1);
});
