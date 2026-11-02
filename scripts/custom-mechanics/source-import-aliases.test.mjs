import test from 'node:test';
import assert from 'node:assert/strict';
import {aliasSourceKeywordFaces,sourceDisplayText} from '../../scripts/custom-mechanics/source-keyword-aliases.mjs';
import {materializeCard,renameSourceKeywords} from '../../scripts/custom-mechanics/cohort-input.mjs';
import {validateMechanic} from '../../web/ui/src/lib/custom-mechanics.js';
import {customKeywordForms} from '../../web/ui/src/lib/keyword-forms.js';
const oldHarness='You may tap a creature you control as you cast this spell. If you do, reduce this spell\'s mana cost by that creature\'s power.';
const harness={keyword:'Draw strength',sourceKeyword:'Harness',sourceReminder:oldHarness,reminder:'As an additional cost to cast this spell, you may tap an untapped creature you control. This spell costs {X} less to cast, where X is the power of the creature tapped this way.'};
test('source alias needs matching reminder and a valid canonical declaration',()=>{
 const source=`Harness (${oldHarness})\nIf this spell was harnessed, draw a card.\nIf this permanent is harnessed, you gain 1 life.\nHarness this permanent.`;
 const result=aliasSourceKeywordFaces([{name:'Legacy spell',oracleText:source}],[harness])[0].oracleText;
 assert.match(result,/^draw strength \(/i);
 const canonical=customKeywordForms(validateMechanic(harness));
 // Multiword custom names inflect the first word and retain the rest; the
 // importer must consume the very same canonical vocabulary as expansion.
 assert.equal(canonical.participle,'drawed strength');
 assert.ok(result.split('\n').includes(`If this spell was ${canonical.participle}, draw a card.`));
 assert.match(result,/this permanent is harnessed/);
 assert.match(result,/Harness this permanent\.$/);
 const official='Harness this permanent. (Once harnessed, its ability is active.)';
 assert.equal(aliasSourceKeywordFaces([{oracleText:official}],[harness])[0].oracleText,official);
 const wrong=`Harness (A different mechanic.)\nIf this spell was harnessed, draw a card.`;
 assert.equal(aliasSourceKeywordFaces([{oracleText:wrong}],[harness])[0].oracleText,wrong);
});
test('nonmana paid declaration migrates while official Ascend on linked face stays official',()=>{
 const old='Tap ten untapped permanents you control: Exile this land, then return it to the battlefield transformed. Ascend only as a sorcery.';
 const definition={keyword:'Exile ascend',sourceKeyword:'Ascend',sourceReminder:old,template:'{Cost}',reminder:'{Cost}: Exile this permanent, then return it to the battlefield transformed. Activate only as a sorcery.'};
 const front=`Ascend—Tap ten untapped permanents you control. (${old})`;
 const back="Ascend (If you control ten or more permanents, you get the city's blessing for the rest of the game.)";
 const result=aliasSourceKeywordFaces([{name:'Before',oracleText:front},{name:'After',oracleText:back}],[definition]);
 assert.match(result[0].oracleText,/^exile ascend/i);
 assert.equal(result[1].oracleText,back);
 assert.equal(sourceDisplayText(front),'Ascend—Tap ten untapped permanents you control.');
});
test('source originals and display remain exact while executable identity is distinct',()=>{
 const original=`Mana cost: {2}{U}\nType: Sorcery\nHarness (${oldHarness})\nReturn target nonland permanent to its owner's hand. If this spell was harnessed, draw a card.`;
 const card={id:'legacy',name:'Legacy spell',original_text:original};
 const before=JSON.stringify(card);
 const [result]=materializeCard(card,[{...validateMechanic(harness),sourceKeyword:harness.sourceKeyword,sourceReminder:harness.sourceReminder}]);
 assert.equal(JSON.stringify(card),before);
 assert.match(result.original_display_text,/^Harness\n/);
 assert.doesNotMatch(result.original_display_text,/draw strength/i);
 assert.deepEqual(result.custom_keyword_identities,['draw strength']);
 const price=result.executable_text.match(/As an additional cost named "([^"]+)" to cast this spell, you may tap an untapped creature you control\./);
 assert.ok(price,'the complete optional additional price retains its named receipt');assert.equal(price[1],'draw strength');
 assert.ok(result.executable_text.includes('This spell costs {X} less to cast, where X is the power of the creature tapped this way'),'the tapped-creature cost reduction remains part of the definition');
 const predicate=result.executable_text.match(/if this spell paid the additional cost named ([^,]+), draw a card\./i);
 assert.ok(predicate,'the paid-status consumer must identify this exact additional price');assert.equal(predicate[1],price[1]);
 assert.deepEqual(result.custom_keyword_payments,[price[1]],'native payment attribution must carry the same canonical label');
 assert.doesNotMatch(result.executable_text,/If this spell's additional cost was paid/i,'an unrelated optional price cannot satisfy this named receipt');
 assert.doesNotMatch(result.executable_text,/\bwas (?:harnessed|drawed strength)\b/i);
 assert.doesNotMatch(result.executable_text,/undefined/);
});
test('import metadata never exempts an official custom keyword from validation',()=>{
 assert.throws(()=>aliasSourceKeywordFaces([{oracleText:`Harness (${oldHarness})`}],[{...harness,keyword:'Harness',sourceScope:'revolution'}]),/official keywords cannot be redefined/);
});

test('an explicitly reminded repeated action imports every repeated source declaration',()=>{
 const reminder='Create a 0/0 white Soldier Army creature token, then put a +1/+1 counter on each Army you control.';
 const definition={keyword:'Muster ranks',sourceKeyword:'Recruit',sourceReminder:reminder,reminder};
 const source=`When this creature enters, recruit, then recruit again. (${reminder})`;
 const [result]=aliasSourceKeywordFaces([{name:'Two musters',oracleText:source}],[definition]);
 assert.match(result.oracleText,/muster ranks, then muster ranks again/i);
});

test('source import rewrites executable granted quotes while preserving printed-text and name operands', () => {
 const original = 'Draw a card.';
 const definition = {keyword:'Sky step', sourceKeyword:'Recall', sourceReminder:original, reminder:original};
 const source = [
  'Creatures you control have "Recall (Draw a card.)".',
  'First mate — Each nonland card in your starting deck has "Recall" or "recalled spells" in its printed rules text.',
  'Choose a card named "Recall".',
 ].join('\n');
 const output = renameSourceKeywords(source,[definition]);
 assert.match(output,/have "sky step \(Draw a card\.\)"/i);
 assert.match(output,/has "Recall" or "recalled spells" in its printed rules text/);
 assert.match(output,/named "Recall"/);
});
