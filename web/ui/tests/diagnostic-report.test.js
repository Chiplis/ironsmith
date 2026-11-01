import test from 'node:test';
import assert from 'node:assert/strict';
import {parseDiagnosticReport} from '../src/lib/diagnostic-report.js';
test('loads exported diagnostic data without dropping snapshots or journals', () => {
 const report={exportedAt:'2026-10-31',mainThread:{lagMs:0},events:[],traces:[],gameState:{turn_number:4},journal:{entries:[{method:'dispatch',args:[{}]}]}};
 assert.deepEqual(parseDiagnosticReport(JSON.stringify(report)),report);
});
test('rejects malformed JSON and unrelated reports',()=>{
 for (const text of ['{','null','[]','{}','{"events":[],"traces":[]}']) assert.throws(()=>parseDiagnosticReport(text));
});
