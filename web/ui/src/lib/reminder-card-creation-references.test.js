import test from 'node:test';
import assert from 'node:assert/strict';
import {expandCreatedCardReferences as expand} from './reminder-card-creation-references.js';
const config={label:'making',verb:'make',past:'made',sourceName:'Memory Keeper'};
test('same-name completion keeps the authored object reference including noncard spellcopies',()=>{
 assert.equal(expand("Whenever you cast a spell, if you've made a card with the same name as it this turn, draw a card.",config),'Whenever you cast a spell, if it is an object sharing a name with a card from your making action this turn, draw a card.');
 assert.equal(expand('A card named "you have made a card with the same name as it this turn".',config),'A card named "you have made a card with the same name as it this turn".');
});
test('singular participant receipt and exact source atom bind note/read programs',()=>{
 const source="Whenever a card participates in the making action performed by you, if its name isn't noted for Memory Keeper, you may note its name.\nSpells you cast with a name noted for Memory Keeper cost one mana of any color less to cast.";
 assert.equal(expand(source,config),"Whenever a card participates in the making action performed by you, if the triggering card's name isn't noted for this permanent, you may note the triggering card's name for this permanent.\nSpells you cast with a name noted for this permanent cost one mana of any color less to cast.");
});
test('longer source names and quoted rules never become this source memory',()=>{
 const source="Whenever a card participates in the making action performed by you, if its name isn't noted for Other Memory Keeper, you may note its name.\nSpells you cast with a name noted for Other Memory Keeper cost one mana of any color less to cast.";
 assert.equal(expand(source,config),source);
 const quoted='This creature has "Spells you cast with a name noted for Memory Keeper cost one mana of any color less to cast."';
 assert.equal(expand(quoted,config),quoted);
});
