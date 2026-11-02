import test from 'node:test';
import assert from 'node:assert/strict';
import {expandGrantedTriggeredPrograms} from '../src/lib/reminder-ability-grants.js';
const definition={keyword:'bruise',reminder:'Damage dealt to this creature also causes its controller to lose that much life.',expansion:'Damage dealt to this creature also causes its controller to lose that much life.'};
test('official or composed custom grants keep a grouped resolution alternative',()=>{
 const text=expandGrantedTriggeredPrograms('Target creature gains lifelink or bruise until end of turn.',[definition]);
 assert.equal(text,'Target creature gains lifelink or custom keyword "bruise", and "Damage dealt to this creature also causes its controller to lose that much life." until end of turn.');
});
test('composed first alternative preserves the following official alternative',()=>{
 const text=expandGrantedTriggeredPrograms('Target creature gains bruise or lifelink until end of turn.',[definition]);
 assert.match(text,/gains custom keyword "bruise", and "[^"\n]+" or lifelink until end of turn/);
});
