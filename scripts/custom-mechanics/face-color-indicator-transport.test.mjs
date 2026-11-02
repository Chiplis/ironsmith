import test from 'node:test';import assert from 'node:assert/strict';import {materializeCard} from './cohort-input.mjs';
test('mana-less back face retains typed source color through native text transport',()=>{
 const card={name:'Front',source_shape:'doubleface',source_faces:[{name:'Front',original_text:'Mana cost: {W}\nType: Instant\nYou gain 1 life.',color_indicator:['White']},{name:'Back',original_text:'Type: Enchantment — Aura\nEnchant creature',color_indicator:['White']}]};
 const faces=materializeCard(card,[]);assert.equal(faces.length,2);assert.deepEqual(faces[1].color_indicator,['White']);assert.match(faces[1].executable_text,/Color indicator: White/);assert.equal(card.source_faces[1].original_text,'Type: Enchantment — Aura\nEnchant creature');
});
test('empty source color metadata remains recorded without inventing an indicator',()=>{
 const faces=materializeCard({name:'Colorless witness',source_faces:[{name:'Colorless witness',original_text:'Type: Enchantment',color_indicator:[]}]},[]);assert.doesNotMatch(faces[0].executable_text,/Color indicator:/);assert.deepEqual(faces[0].color_indicator,[]);
});

test('linked face colors and supplied token programs share one source-preserving transport',()=>{
 const token={name:'Keepsake',manaCost:null,cardTypes:['Artifact'],colorIndicator:['Blue'],oracleText:'{T}: Add {C}.'};
 const card={name:'Color token creator',source_shape:'doubleface',scope:'msem',custom_tokens:[{face:token,input:structuredClone(token)}],source_faces:[{name:'Color token creator',original_text:'Mana cost: {W}\nType: Instant\nCreate a Keepsake token.',color_indicator:['White']},{name:'Colored back Aura',original_text:'Type: Enchantment — Aura\nEnchant creature',color_indicator:['White']}]};
 const before=structuredClone(card);const faces=materializeCard(card,[],'msem');
 assert.match(faces[0].executable_text,/Color indicator: White/);assert.match(faces[0].executable_text,/token named Keepsake/);assert.match(faces[1].executable_text,/Color indicator: White/);assert.deepEqual(faces[1].color_indicator,['White']);
 for(const face of faces){assert.equal(face.custom_token_definitions.length,1);assert.equal(face.custom_token_definitions[0].name,'Keepsake');assert.match(face.custom_token_definitions[0].executable_text,/Color indicator: Blue/);assert.match(face.custom_token_definitions[0].executable_text,/\{T\}: Add \{C\}\./);}
 assert.deepEqual(card,before);
});
