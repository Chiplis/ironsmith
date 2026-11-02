import test from 'node:test';
import assert from 'node:assert/strict';
import {compileDraftInput} from '../../web/ui/src/lib/custom-mechanics.js';
const defs=[{keyword:'Judge {Object}',actionProgram:true,reminder:'You choose honor or condemn. If you chose honor, perform the honor action with {Object}, then put a +1/+1 counter on {Object}. If you chose condemn, perform the condemn action with {Object}, then sacrifice {Object}, then draw a card.'}];
for(const [name,source,first] of [
 ['Fall from the Heavens','Destroy target creature with flying. Judge up to one target creature you control.','Destroy target creature with flying.'],
 ['Ordeal of Kaneh','Target creature gets -5/-5 until end of turn. Judge up to one target creature you control.','Target creature gets -5/-5 until end of turn.'],
])test(`${name} bounds the whole optional recipient action after its mandatory sibling`,()=>{
 const face=compileDraftInput({faces:[{name,oracleText:source}]},defs).faces[0];
 assert.ok(face.oracleText.startsWith(first),face.oracleText);
 assert.match(face.oracleText,/Choose up to one target creature you control\. For each permanent chosen this way, do the following: \[/i);
 assert.match(face.oracleText,/\[Perform the Judge action as follows:.*draw a card/i);
 assert.doesNotMatch(face.oracleText,/Judge up to one target/i);
 assert.equal(face.displayOracleText,source);
});
test('the optional recipient scope does not absorb a following sibling or quoted literal',()=>{
 const source='Judge up to one target creature you control. You gain 2 life. Create a 1/1 white Soldier creature token named "Judge up to one target creature you control."';
 const result=compileDraftInput({faces:[{name:'Bounded recipient',oracleText:source}]},defs).faces[0].oracleText;
 assert.match(result,/\]\. You gain 2 life\./i);
 assert.ok(result.includes('named "Judge up to one target creature you control."'),result);
});
