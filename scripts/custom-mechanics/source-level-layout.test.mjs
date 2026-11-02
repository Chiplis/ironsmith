import test from 'node:test';
import assert from 'node:assert/strict';
import {materializeSourceLevelLayout,materializeCard} from './cohort-input.mjs';
test('source level bands and printed statistics become independent native layout fields',()=>{
 const metadata=['Mana cost: {G}{W}','Type: Artifact Creature — Elephant Construct'];
 const lines=['Whenever this creature attacks, draw a card.','When this creature has three level counters on it, transform it. [2/2]','[level 1-3] Vigilance [3/3]'];
 const result=materializeSourceLevelLayout(metadata,lines);
 assert.deepEqual(result.metadata,[...metadata,'Power/Toughness: 2/2']);
 assert.deepEqual(result.lines,['Whenever this creature attacks, draw a card.','When this creature has three level counters on it, transform it.','LEVEL 1-3','Vigilance','3/3']);
 assert.equal(lines[1].endsWith('[2/2]'),true);
});
test('nonlevel creature statistics become base metadata and quoted display text is preserved',()=>{
 const result=materializeSourceLevelLayout(['Type: Creature — Beast'],['Flying [5/5]']);
 assert.deepEqual(result,{metadata:['Type: Creature — Beast','Power/Toughness: 5/5'],lines:['Flying']});
 assert.deepEqual(materializeSourceLevelLayout(['Type: Enchantment'],['The chosen label is "[2/2]".']).lines,['The chosen label is "[2/2]".']);
});
test('only complete attached token display blocks are separated from main oracle',async()=>{
 const {separateSourceTokenAppendices}=await import('./cohort-input.mjs');
 const text='Equipped creature gets +1/+1.\n---\n**Named token, with a comma**\nToken Legendary Creature — Demon\nFlying\n**3/2**';
 const result=separateSourceTokenAppendices(text);
 assert.equal(result.text,'Equipped creature gets +1/+1.');assert.equal(result.auxiliary[0].name,'Named token, with a comma');assert.match(result.auxiliary[0].executable_text,/Power\/Toughness: 3\/2/);assert.equal(result.auxiliary[0].original_text,text.split('\n---\n')[1]);
 const incomplete='Draw a card.\n---\n**Unbound name**';assert.equal(separateSourceTokenAppendices(incomplete).text,incomplete);
});

test('trailing source P/T fields remain card metadata rather than executable rules',()=>{
 const original='Mana cost: {1}{U}\nType: Creature — Wizard\nFlying\nP/T: 1/3';
 const card={name:'Independent export',original_text:original,source_shape:'normal'};
 const [face]=materializeCard(card,[],'msem');
 assert.equal(face.executable_text,'Mana cost: {1}{U}\nType: Creature — Wizard\nPower/Toughness: 1/3\nFlying');
 assert.equal(card.original_text,original);
});

test('materializing host statistics preserves independent appended token statistics',()=>{
 const original='Type: Creature\nFlying\nP/T: 1/1\n---\n**Named token**\nType: Creature\nP/T: 3/3';
 const card={name:'Appendix source',original_text:original,source_shape:'normal'};
 const [face]=materializeCard(card,[],'msem');
 assert.match(face.executable_text,/Power\/Toughness: 1\/1\nFlying/);
 assert.equal((face.executable_text.match(/Power\/Toughness:/g)||[]).length,1);
 assert.match(face.executable_text,/---[\s\S]*P\/T: 3\/3/);
 assert.ok(face.executable_text.includes('---\n**Named token**\nType: Creature\nP/T: 3/3'));
 assert.equal(card.original_text,original);
});
