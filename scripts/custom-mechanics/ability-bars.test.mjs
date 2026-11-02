import test from 'node:test';
import assert from 'node:assert/strict';
import {readAbilityBarProgram, expandAbilityBarProgram} from '../../web/ui/src/lib/reminder-ability-bars.js';
import {compileDraftInput,validateMechanic} from '../../web/ui/src/lib/custom-mechanics.js';
const reminder = 'Ability bars are activated abilities. Each bar costs its printed cost. Each bar adds its printed subtype. Each bar puts a +1/+1 counter on this permanent. Each bar performs its printed effect. Activate bars only once. Activate bars only as a sorcery. Activate bars in printed order.';
test('bar reader composes independent instructions under arbitrary declaration names', () => {
 const def = validateMechanic({keyword:'Ascension witness', reminder});
 const text='Flying\nAscension witness\n{U}: Wizard — Draw a card.\n{2}{U}: God — You gain 3 life.';
 const expanded=compileDraftInput({faces:[{name:'Witness',oracleText:text}]},[def]).faces[0];
 assert.equal(expanded.oracleText.replace(/Family \d+ /g, ''),'Flying\nSequential group 0 step 0 — {U}: This permanent becomes a Wizard in addition to its other types. Put a +1/+1 counter on this permanent. Draw a card. Activate only as a sorcery.\nSequential group 0 step 1 — {2}{U}: This permanent becomes a God in addition to its other types. Put a +1/+1 counter on this permanent. You gain 3 life. Activate only as a sorcery.');
 assert.equal(expanded.displayOracleText,text);
});
test('ordinary top-box abilities do not become group members and costs remain verbatim',()=>{
 const text='{T}: Add {U}.\nWitness\n{1}, Sacrifice a Food: God — Draw a card.';
 const expanded=expandAbilityBarProgram(text,{keyword:'Witness',reminder});
 assert.ok(expanded.startsWith('{T}: Add {U}.\nFamily '));
 assert.ok(expanded.includes('— {1}, Sacrifice a Food:'));
});
test('missing cost/body and unknown rules fail explicitly instead of losing semantics',()=>{
 assert.throws(()=>expandAbilityBarProgram('Witness\nFlying',{keyword:'Witness',reminder}));
 assert.throws(()=>readAbilityBarProgram(reminder+' Draw a card.'));
 assert.equal(readAbilityBarProgram('Draw a card.'),null);
});
test('bar sections preserve multiline source effects',()=>{
 const expanded=expandAbilityBarProgram('Witness\n{U}: Wizard\nDraw a card.\nYou gain 1 life.\n{2}{U}: God\nYou gain 3 life.',{keyword:'Witness',reminder});
 assert.match(expanded,/Draw a card\. You gain 1 life\. Activate only as a sorcery/);
 assert.match(expanded,/step 1 — \{2\}\{U\}: This permanent becomes a God/);
});
test('complete source cohort retains Shi Hou linked back face and every source variant',async()=>{
 const {readFile}=await import('node:fs/promises');
 const fixture=JSON.parse(await readFile(new URL('../../fixtures/custom_mechanics/msem-category-bars.json',import.meta.url),'utf8'));
 assert.equal(fixture.mechanics[0].representative_cards.length,27);
 assert.equal(fixture.mechanics[1].representative_cards.length,21);
 const shiHou=fixture.mechanics[0].representative_cards.find(card=>card.name==='Shi Hou, the Sun King');
 assert.equal(shiHou.source_faces.length,2);
 assert.equal(shiHou.executable_faces.length,2);
 assert.equal(shiHou.executable_faces[1].name,"Sun King's Shadow");
 assert.equal(shiHou.executable_faces[1].executable_text,shiHou.source_faces[1].original_text);
});
