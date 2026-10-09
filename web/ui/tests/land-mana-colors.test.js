import test from 'node:test';
import assert from 'node:assert/strict';
import {landManaColors} from '../src/lib/land-mana-colors.js';
const land = (overrides={})=>({type_line:'Land',...overrides});
test('all production colors are unique and ordered, including colorless',()=>{
 assert.deepEqual(landManaColors(land({produced_mana:['G','W','C','U','B','R','G']})),['W','U','B','R','G','C']);
});
test('tapped dual lands and animated lands retain their mana border',()=>{
 assert.deepEqual(landManaColors(land({type_line:'Land — Plains Island',tapped:true})),['W','U']);
 assert.deepEqual(landManaColors(land({type_line:'Land Creature',produced_mana:['G']})),['G']);
});
test('cost colors and fetch abilities are not mana production',()=>{
 assert.deepEqual(landManaColors(land({oracle_text:'{W}, {T}: Add {C}{C}.'})),['C']);
 assert.deepEqual(landManaColors(land({oracle_text:'{T}, Pay 1 life, Sacrifice this land: Search your library for a Forest card.'})),[]);
 assert.deepEqual(landManaColors(land({type_line:'Land — Forest',produced_mana:[]})),[]);
 assert.deepEqual(landManaColors({type_line:'Artifact',produced_mana:['C']}),[]);
});
test('rainbow mana is not truncated to two colors',()=>{
 assert.deepEqual(landManaColors(land({oracle_text:'{T}: Add one mana of any color.'})),['W','U','B','R','G']);
});

test('live permanent snapshots use lane and compiled abilities rather than type_line',()=>{
 assert.deepEqual(landManaColors({lane:'land',oracle_text:'{T}, Pay 1 life: Add {U} or {R}.',produced_mana:[]}),['U','R']);
 assert.deepEqual(landManaColors({lane:'lands',abilities:['{T}: Add {G}.']}),['G']);
 assert.deepEqual(landManaColors({lane:'land',oracle_text:'{T}, Pay 1 life, Sacrifice this land: Search your library for a Mountain or Plains card.'}),[]);
});
