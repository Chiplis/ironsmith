import test from "node:test";
import assert from "node:assert/strict";
import { initialCounterDraft, parseCounterAllocationChoice, updateCounterDraft } from "../src/lib/counter-choice.js";
import { isDecisionCommandCompatible, resolveSyncedCommand } from "../src/lib/sync-commands.js";
import { decisionKey } from "../src/lib/decision-key.js";
const maximum = 4294967295;
const decision = (min="0",max="8589934590") => ({kind:"select_counters", min_total:min,max_total:max,options:[{index:0,max_count:maximum,legal:true},{index:1,max_count:maximum,legal:true}]});
test("unsigned per-kind quantities and aggregate stay sparse through peer wire", () => {
  const d=decision("8589934590"); const draft=initialCounterDraft(d);
  const selected=parseCounterAllocationChoice(d,draft);
  assert.equal(selected.total,"8589934590");assert.equal(selected.command.allocations.length,2);
  assert.deepEqual(resolveSyncedCommand(JSON.parse(JSON.stringify(selected.command))),selected.command);
  assert.equal(isDecisionCommandCompatible(d,selected.command),true);
});
test("minimum and aggregate maximum are enforced without signed coercion", () => {
  assert.equal(parseCounterAllocationChoice(decision("3","5"),[{index:0,value:"2"}]),null);
  assert.equal(parseCounterAllocationChoice(decision("3","5"),[{index:0,value:"6"}]),null);
  assert.equal(parseCounterAllocationChoice(decision("3","5"),[{index:0,value:"3"}]).total,"3");
});
test("all unsigned boundary quantities remain valid", () => {
  for(const amount of [0,2147483646,2147483647,2147483648,4294967294,maximum]) {
    const parsed=parseCounterAllocationChoice(decision(),[{index:0,value:String(amount)}]);
    assert.equal(parsed.total,String(amount));assert.deepEqual(parsed.command.allocations,amount?[{index:0,count:amount}]:[]);
  }
});
test("malformed, unavailable, duplicate and oversized quantities reject", () => {
  for(const value of ["-1","1.5","1e3","",String(maximum+1)]) assert.equal(parseCounterAllocationChoice(decision(),[{index:0,value}]),null);
  assert.equal(parseCounterAllocationChoice(decision(),[{index:3,value:"1"}]),null);
  assert.equal(parseCounterAllocationChoice(decision(),[{index:0,value:"1"},{index:0,value:"1"}]),null);
  const d=decision();d.options[0].legal=false;assert.equal(parseCounterAllocationChoice(d,[{index:0,value:"1"}]),null);
  assert.throws(()=>resolveSyncedCommand({type:"select_counters",allocations:[{index:0,count:maximum+1}]}));
});
test("quantity selection order reaches the command unchanged", () => {
  let draft=initialCounterDraft(decision());draft=updateCounterDraft(draft,1,"3");draft=updateCounterDraft(draft,0,"2");draft=updateCounterDraft(draft,1,"4");
  assert.deepEqual(parseCounterAllocationChoice(decision(),draft).command.allocations,[{index:1,count:4},{index:0,count:2}]);
});
test("changed limits or capacity reset the decision draft identity", () => {
  const d=decision();assert.notEqual(decisionKey(d),decisionKey({...d,min_total:"1"}));
  assert.notEqual(decisionKey(d),decisionKey({...d,options:[{...d.options[0],max_count:4},d.options[1]]}));
});
test("large portable decimal bounds use exact integer comparisons", () => {
  assert.equal(parseCounterAllocationChoice(decision("9007199254740993","18446744073709551615"),[{index:0,value:"1"}]),null);
  assert.equal(parseCounterAllocationChoice(decision("0","18446744073709551616"),[]),null);
});

test("clearing both quantity fields retains first positive selection order", () => {
  let draft=initialCounterDraft(decision());draft=updateCounterDraft(draft,1,"");draft=updateCounterDraft(draft,1,"3");draft=updateCounterDraft(draft,0,"");draft=updateCounterDraft(draft,0,"2");
  assert.deepEqual(parseCounterAllocationChoice(decision(),draft).command.allocations,[{index:1,count:3},{index:0,count:2}]);
});
