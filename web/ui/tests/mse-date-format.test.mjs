import test from 'node:test';
import assert from 'node:assert/strict';
import {formatMseDateTime} from '../src/lib/mse/date-format.mjs';
import {MseDateTime} from '../src/lib/mse/script-values.mjs';
import {MseRuntime} from '../src/lib/mse/script-runtime.mjs';
import {coreBuiltins} from '../src/lib/mse/script-builtins.mjs';
const value=(y,m,d,h=0,min=0,s=0,ms=0)=>new MseDateTime(new Date(y,m-1,d,h,min,s,ms));
test('date formatting supports template timestamps, literal text and milliseconds',()=>{
  const date=value(2024,2,29,13,4,5,123);
  assert.equal(formatMseDateTime(date,'%S%M%H%d%m%y'),'050413290224');
  assert.equal(formatMseDateTime(date,'%y-%m-%d, %H:%M:%S'),'24-02-29, 13:04:05');
  assert.equal(formatMseDateTime(date,'%% %F %r %j %l'),'% 2024-02-29 01:04:05 PM 060 123');
});
test('week numbering handles years whose first days belong to the previous ISO year',()=>{
  assert.equal(formatMseDateTime(value(2021,1,1),'%G-%V-%u %U %W'),'2020-53-5 00 00');
  assert.equal(formatMseDateTime(value(2021,1,3),'%G-%V-%u %U %W'),'2020-53-7 01 00');
  assert.equal(formatMseDateTime(value(2021,1,4),'%G-%V-%u %U %W'),'2021-01-1 01 01');
});
test('to_string dispatches dates before numeric formats and retains delayed errors',()=>{
  const runtime=new MseRuntime({builtins:coreBuiltins()});runtime.scope.set('stamp',value(2025,6,7));
  assert.equal(runtime.run('to_string(stamp,format:"%d")'),'07');
  assert.equal(runtime.run('to_string(stamp,format:"%Q") or else "fallback"'),'fallback');
  assert.equal(runtime.run('to_string(7,format:"%d")'),'7');
});

test('format is a callable builtin sharing date and numeric formatting with to_string',()=>{
  const runtime=new MseRuntime({builtins:coreBuiltins()});runtime.scope.set('stamp',value(2025,6,7,13,4,5));
  assert.equal(runtime.run('format(stamp,format:"%Y-%m-%d")'),'2025-06-07');
  assert.equal(runtime.run('format(30,format:".3f")'),'30.000');
  assert.equal(runtime.run('format(30,format:"X")'),'1E');
  assert.equal(runtime.run('format("hello",format:"s")'),'hello');
  assert.equal(runtime.run('to_string(30)'),'30');
  assert.throws(()=>runtime.run('format(stamp,format:"%Q")'),{code:'script-format'});
});
