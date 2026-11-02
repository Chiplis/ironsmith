import test from 'node:test';
import assert from 'node:assert/strict';
import {aliasSourceKeywordFaces} from './source-keyword-aliases.mjs';
import {validateMechanic} from '../../web/ui/src/lib/custom-mechanics.js';
const definition={keyword:'Tune ensemble',sourceKeyword:'Harmonize',sourceReminders:['Trigger up to one harmony of a permanent you control.'],reminder:'Trigger up to one harmony triggered ability of permanents you control as the tune ensemble action.',actionProgram:true};
const canonical=validateMechanic(definition).keyword;
const alias=(oracleText,defs=[definition])=>aliasSourceKeywordFaces([{name:'Reference-only source',oracleText}],defs)[0].oracleText;
test('explicit action references alias without inventing an instruction on the same card',()=>{
 assert.equal(canonical,'tune ensemble','source import uses the strict validator canonical name');
 assert.equal(alias('If you would harmonize while this spell is on the stack, you may instead put three thunder counters on this spell.'),`If you would ${canonical} while this spell is on the stack, you may instead put three thunder counters on this spell.`);
 assert.equal(alias('As you harmonize, you may choose another harmony ability to trigger.'),`As you ${canonical}, you may choose another harmony ability to trigger.`);
});
test('action-reference aliases retain exact evidence, declared-program requirement, and literal opacity',()=>{
 const literal='Each nonland card has “harmonize” in its printed rules text.';assert.equal(alias(literal),literal);
 const named='Search your library for a card named "If you would harmonize, draw a card.".';assert.equal(alias(named),named);
 const ordinary='If you would harmonize, instead draw a card.';assert.equal(alias(ordinary,[{...definition,actionProgram:false}]),ordinary);
 assert.throws(()=>alias(ordinary,[{...definition,sourceReminders:[]}]),/exact original reminder evidence/);
});
